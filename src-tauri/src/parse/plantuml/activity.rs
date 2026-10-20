//! Importador de diagramas de actividad de PlantUML.
//!
//! Es el unico de los seis dialectos que no se escribe como una lista de
//! relaciones. Un diagrama de actividad es una **secuencia de sentencias**
//! —`start`, `:accion;`, `if/else/endif`, `fork`— donde el orden del texto es
//! lo que dice por donde sigue el flujo. Nadie escribe las flechas; hay que
//! deducirlas.
//!
//! Por eso la importacion va en dos pasos:
//!
//! 1. `Reader` convierte el texto en un arbol de sentencias. Aqui se cierran
//!    los bloques y se avisa de lo que no entra.
//! 2. `Builder` recorre ese arbol construyendo el grafo, llevando en la mano
//!    la lista de **extremos sueltos**: los nodos de los que todavia tiene que
//!    salir una flecha hacia lo que venga despues. Una accion deja exactamente
//!    uno, un `if` sin `else` deja dos, un `stop` no deja ninguno. Cuando
//!    aparece la siguiente sentencia, todos los extremos sueltos apuntan a
//!    ella y la lista se renueva.
//!
//! Esa lista es todo el truco. Con ella un `if/else` anidado dentro de un
//! `while` dentro de un `fork` sale solo, sin ningun caso especial.
//!
//! Lo que no entra, y avisa en vez de callarse: la sintaxis antigua
//! (`(*) --> "algo"`), las notas, `detach`/`kill` y las flechas con etiqueta
//! propia (`-> texto;`).

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, Node, NodeShape, Routing, Span,
};

use super::super::ImportResult;
use super::elements::{read_token, strip_word};

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut reader = Reader::new(lines_of(source));
    let body = reader.block(&[]);
    let (diagnostics, title) = reader.finish();

    let mut builder = Builder::new(now_ms, diagnostics);
    if let Some(title) = title {
        builder.document.meta.title = title;
    }
    builder.walk(&body, Vec::new());
    builder.finish()
}

/* --- Sentencias ------------------------------------------------------------ */

#[derive(Debug)]
enum Stmt {
    Action(String),
    Start,
    Stop,
    If(Conditional),
    While(WhileLoop),
    Repeat(RepeatLoop),
    Parallel(Parallel),
    Partition { name: String, body: Vec<Stmt> },
}

/// Un `if` con sus `elseif` y su `else`.
#[derive(Debug)]
struct Conditional {
    branches: Vec<Branch>,
    /// La rama `else`, con la etiqueta de la flecha que lleva a ella.
    otherwise: Option<(Option<String>, Vec<Stmt>)>,
}

#[derive(Debug)]
struct Branch {
    condition: String,
    /// Etiqueta de la flecha que entra a la rama (`then (si)`).
    label: Option<String>,
    body: Vec<Stmt>,
}

/// `while (c) is (si) ... endwhile (no)`: la condicion va delante, asi que el
/// cuerpo puede no ejecutarse nunca.
#[derive(Debug)]
struct WhileLoop {
    condition: String,
    inner: Option<String>,
    outer: Option<String>,
    body: Vec<Stmt>,
}

/// `repeat ... repeat while (c)`: la condicion va detras, asi que el cuerpo se
/// ejecuta al menos una vez.
#[derive(Debug)]
struct RepeatLoop {
    body: Vec<Stmt>,
    condition: String,
    inner: Option<String>,
    outer: Option<String>,
}

#[derive(Debug)]
struct Parallel {
    branches: Vec<Vec<Stmt>>,
    /// `fork` dibuja barras de sincronizacion; `split` reparte sin ellas.
    bars: bool,
    /// `end fork` vuelve a unir; `end merge` deja los extremos sueltos.
    joins: bool,
}

/* --- Lectura de lineas ----------------------------------------------------- */

/// Una linea util, ya sin comentarios y con su sitio en el texto.
struct Line {
    text: String,
    line: u32,
    column: u32,
}

/// Signos con los que PlantUML cierra una accion. El primero es el normal; los
/// demas cambian la forma de la caja, que aqui no se distingue.
const ACTION_END: &[char] = &[';', '|', '<', '>', '/', ']', '}'];

/// Parte el texto en lineas utiles, juntando las acciones que ocupan varias.
///
/// Una accion puede escribirse en varios renglones hasta su `;`, y dentro de
/// ella un salto de linea es parte del texto, no el final de la sentencia.
fn lines_of(source: &str) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut pending: Option<Line> = None;
    let mut in_comment = false;

    for (index, raw) in source.lines().enumerate() {
        let number = index as u32 + 1;
        let text = raw.trim();
        let column = (raw.len() - raw.trim_start().len()) as u32 + 1;

        if in_comment {
            if text.contains("'/") {
                in_comment = false;
            }
            continue;
        }
        if text.starts_with("/'") {
            in_comment = !text.contains("'/");
            continue;
        }

        // Dentro de una accion a medias no se filtra nada: lo que haya es
        // texto de la accion.
        if let Some(open) = &mut pending {
            open.text.push('\n');
            open.text.push_str(text);
            if text.ends_with(ACTION_END) {
                out.push(pending.take().unwrap());
            }
            continue;
        }

        if text.is_empty() || text.starts_with('\'') {
            continue;
        }

        let open = Line {
            text: text.to_string(),
            line: number,
            column,
        };

        if text.starts_with(':') && !text.ends_with(ACTION_END) {
            pending = Some(open);
        } else {
            out.push(open);
        }
    }

    // Una accion sin cerrar se queda igualmente: el lector avisara.
    if let Some(open) = pending {
        out.push(open);
    }

    out
}

/* --- Lector ---------------------------------------------------------------- */

/// Que clase de sentencia abre una linea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Action,
    Start,
    Stop,
    If,
    ElseIf,
    Else,
    EndIf,
    While,
    EndWhile,
    Repeat,
    RepeatWhile,
    Fork,
    ForkAgain,
    EndFork,
    EndMerge,
    Split,
    SplitAgain,
    EndSplit,
    Partition,
    Close,
    Other,
}

/// La clase se decide por el principio de la linea, y el orden de las pruebas
/// importa: `end fork` antes que `fork`, `repeat while` antes que `repeat`.
fn kind_of(text: &str) -> Kind {
    let flat = flatten(text);

    if text.starts_with(':') {
        return Kind::Action;
    }

    for (written, kind) in [
        ("end fork", Kind::EndFork),
        ("endfork", Kind::EndFork),
        ("end merge", Kind::EndMerge),
        ("endmerge", Kind::EndMerge),
        ("end split", Kind::EndSplit),
        ("endsplit", Kind::EndSplit),
        ("fork again", Kind::ForkAgain),
        ("split again", Kind::SplitAgain),
        ("repeat while", Kind::RepeatWhile),
        ("else if", Kind::ElseIf),
        ("elseif", Kind::ElseIf),
        ("endif", Kind::EndIf),
        ("end if", Kind::EndIf),
        ("endwhile", Kind::EndWhile),
        ("end while", Kind::EndWhile),
    ] {
        if flat == written || flat.starts_with(&format!("{written} ")) || opens(&flat, written) {
            return kind;
        }
    }

    for (written, kind) in [
        ("if", Kind::If),
        ("while", Kind::While),
        ("else", Kind::Else),
        ("fork", Kind::Fork),
        ("split", Kind::Split),
        ("repeat", Kind::Repeat),
        ("partition", Kind::Partition),
        ("group", Kind::Partition),
    ] {
        if flat == written || flat.starts_with(&format!("{written} ")) || opens(&flat, written) {
            return kind;
        }
    }

    match flat.as_str() {
        "start" => Kind::Start,
        "stop" | "end" => Kind::Stop,
        "}" => Kind::Close,
        _ => Kind::Other,
    }
}

/// `true` si la palabra va pegada a su parentesis (`if(c)`, que PlantUML
/// acepta igual que `if (c)`).
fn opens(flat: &str, word: &str) -> bool {
    flat.starts_with(&format!("{word}("))
}

/// Minusculas y un solo espacio entre palabras.
fn flatten(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lee un `(...)` al principio del texto. Cuenta parentesis, porque una
/// condicion puede llevarlos dentro.
fn paren(text: &str) -> Option<(String, &str)> {
    let text = text.trim_start();
    let rest = text.strip_prefix('(')?;

    let mut depth = 1usize;
    for (at, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((rest[..at].trim().to_string(), &rest[at + 1..]));
                }
            }
            _ => {}
        }
    }

    None
}

/// Como `strip_word`, pero admitiendo que la palabra vaya pegada a su
/// parentesis: PlantUML acepta `if(c)` igual que `if (c)`.
fn strip_keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    if let Some(rest) = strip_word(text, word) {
        return Some(rest);
    }

    let head = text.get(..word.len())?;
    if !head.eq_ignore_ascii_case(word) {
        return None;
    }

    let rest = &text[word.len()..];
    rest.starts_with('(').then_some(rest)
}

/// Quita cualquiera de las palabras, la primera que encaje.
fn strip_any<'a>(text: &'a str, words: &[&str]) -> Option<&'a str> {
    words.iter().find_map(|word| strip_keyword(text, word))
}

struct Reader {
    lines: Vec<Line>,
    at: usize,
    diagnostics: Vec<Diagnostic>,
    title: Option<String>,
}

impl Reader {
    fn new(lines: Vec<Line>) -> Self {
        Reader {
            lines,
            at: 0,
            diagnostics: Vec::new(),
            title: None,
        }
    }

    fn finish(self) -> (Vec<Diagnostic>, Option<String>) {
        (self.diagnostics, self.title)
    }

    fn peek(&self) -> Option<&Line> {
        self.lines.get(self.at)
    }

    fn next(&mut self) -> Option<&Line> {
        let line = self.lines.get(self.at);
        if line.is_some() {
            self.at += 1;
        }
        line
    }

    fn error(&mut self, code: &str, message: impl Into<String>, line: u32, column: u32) {
        self.diagnostics
            .push(Diagnostic::error(code, message).at_span(Span::new(line, column)));
    }

    fn warn(&mut self, code: &str, message: impl Into<String>, line: u32, column: u32) {
        self.diagnostics
            .push(Diagnostic::warning(code, message).at_span(Span::new(line, column)));
    }

    /// Lee sentencias hasta toparse con una de las clases que cierran el
    /// bloque. Esa clase **no** se consume: la deja para quien abrio.
    fn block(&mut self, closers: &[Kind]) -> Vec<Stmt> {
        let mut out = Vec::new();

        while let Some(line) = self.peek() {
            let kind = kind_of(&line.text);
            if closers.contains(&kind) {
                break;
            }

            let line = self.next().unwrap();
            let (text, number, column) = (line.text.clone(), line.line, line.column);

            match kind {
                Kind::Action => out.push(Stmt::Action(action_text(&text))),
                Kind::Start => out.push(Stmt::Start),
                Kind::Stop => out.push(Stmt::Stop),
                Kind::If => {
                    if let Some(stmt) = self.conditional(&text, number, column) {
                        out.push(stmt);
                    }
                }
                Kind::While => {
                    if let Some(stmt) = self.while_loop(&text, number, column) {
                        out.push(stmt);
                    }
                }
                Kind::Repeat => {
                    if let Some(stmt) = self.repeat_loop(number, column) {
                        out.push(stmt);
                    }
                }
                Kind::Fork | Kind::Split => {
                    if let Some(stmt) = self.parallel(kind, number, column) {
                        out.push(stmt);
                    }
                }
                Kind::Partition => {
                    if let Some(stmt) = self.partition(&text, number, column) {
                        out.push(stmt);
                    }
                }
                // Un cierre que nadie espera: sobra.
                Kind::EndIf
                | Kind::Else
                | Kind::ElseIf
                | Kind::EndWhile
                | Kind::RepeatWhile
                | Kind::ForkAgain
                | Kind::EndFork
                | Kind::EndMerge
                | Kind::SplitAgain
                | Kind::EndSplit
                | Kind::Close => self.error(
                    "unexpectedClose",
                    format!("`{text}` cierra un bloque que no esta abierto."),
                    number,
                    column,
                ),
                Kind::Other => self.other(&text, number, column),
            }
        }

        out
    }

    /// Lineas que no son sentencias: titulo, directivas y lo que no se
    /// entiende.
    fn other(&mut self, text: &str, line: u32, column: u32) {
        let flat = flatten(text);

        if flat.starts_with("@start") || flat.starts_with("@end") {
            return;
        }

        if let Some(rest) = strip_word(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.title = Some(title.to_string());
            }
            return;
        }

        // La sintaxis antigua escribe las flechas a mano. Es otro idioma, no
        // un trozo que falte de este.
        if flat.contains("(*)") || (flat.contains("-->") && !flat.starts_with("->")) {
            self.error(
                "legacyActivity",
                "La sintaxis antigua de actividad (`(*) --> ...`) no se importa.",
                line,
                column,
            );
            return;
        }

        if flat.starts_with("->") {
            self.warn(
                "ignoredArrowLabel",
                "Las etiquetas sueltas de flecha no se importan.",
                line,
                column,
            );
            return;
        }

        if flat == "detach" || flat == "kill" {
            self.warn(
                "ignoredDetach",
                "`detach` y `kill` no se importan: el flujo sigue de largo.",
                line,
                column,
            );
            return;
        }

        if flat.starts_with("backward") {
            self.warn(
                "ignoredBackward",
                "`backward` no se importa: su accion no entra en el bucle.",
                line,
                column,
            );
            return;
        }

        if let Some(code) = ignored(&flat) {
            if flat.starts_with("note") && !text.contains(':') {
                self.skip_note();
            }
            self.warn(code.0, code.1, line, column);
            return;
        }

        self.error(
            "badStatement",
            format!("No se entiende `{text}`."),
            line,
            column,
        );
    }

    /// Se come lo que queda de un bloque que no se pudo leer, contando los
    /// que se abran dentro para no salir por el cierre que no toca.
    fn discard(&mut self, opener: Kind, closer: Kind) {
        let mut depth = 1usize;

        while let Some(line) = self.next() {
            let kind = kind_of(&line.text);
            if kind == opener {
                depth += 1;
            } else if kind == closer {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
        }
    }

    /// Se come una nota de varias lineas: lo de dentro no es del diagrama.
    fn skip_note(&mut self) {
        while let Some(line) = self.next() {
            if flatten(&line.text).starts_with("end note") {
                return;
            }
        }
    }

    fn conditional(&mut self, text: &str, line: u32, column: u32) -> Option<Stmt> {
        const CLOSERS: &[Kind] = &[Kind::ElseIf, Kind::Else, Kind::EndIf];

        let mut branches = Vec::new();
        let Some((condition, label)) = self.condition(text, &["if"], &["then"], line, column)
        else {
            // Sin condicion no hay rombo, pero el bloque sigue ahi: hay que
            // comerselo entero o su `endif` suelta un segundo error que solo
            // es eco del primero.
            self.discard(Kind::If, Kind::EndIf);
            return None;
        };
        branches.push(Branch {
            condition,
            label,
            body: self.block(CLOSERS),
        });

        let mut otherwise = None;

        loop {
            let Some(next) = self.peek() else {
                self.error(
                    "unterminatedIf",
                    "Falta el `endif` que cierra el `if`.",
                    line,
                    column,
                );
                break;
            };

            let kind = kind_of(&next.text);
            let next = self.next().unwrap();
            let (text, number, column) = (next.text.clone(), next.line, next.column);

            match kind {
                Kind::ElseIf => {
                    let Some((condition, label)) =
                        self.condition(&text, &["elseif", "else"], &["if", "then"], number, column)
                    else {
                        self.discard(Kind::If, Kind::EndIf);
                        break;
                    };
                    branches.push(Branch {
                        condition,
                        label,
                        body: self.block(CLOSERS),
                    });
                }
                Kind::Else => {
                    let label = strip_word(&text, "else")
                        .and_then(paren)
                        .map(|(label, _)| label)
                        .filter(|l| !l.is_empty());
                    otherwise = Some((label, self.block(&[Kind::EndIf])));
                }
                _ => break,
            }
        }

        Some(Stmt::If(Conditional {
            branches,
            otherwise,
        }))
    }

    /// Lee `palabra (condicion) union (etiqueta)`, que es la forma de un `if`
    /// y la de un `while`.
    fn condition(
        &mut self,
        text: &str,
        words: &[&str],
        joiners: &[&str],
        line: u32,
        column: u32,
    ) -> Option<(String, Option<String>)> {
        let mut rest = strip_any(text, words)?;

        // `else if (c)` lleva dos palabras; `elseif (c)`, una.
        if let Some(after) = strip_any(rest.trim_start(), joiners)
            && paren(after).is_some()
        {
            rest = after;
        }

        let Some((condition, after)) = paren(rest) else {
            self.error(
                "missingCondition",
                format!("Falta la condicion entre parentesis en `{text}`."),
                line,
                column,
            );
            return None;
        };

        let label = strip_any(after.trim_start(), joiners)
            .and_then(paren)
            .map(|(label, _)| label)
            .filter(|l| !l.is_empty());

        Some((condition, label))
    }

    fn while_loop(&mut self, text: &str, line: u32, column: u32) -> Option<Stmt> {
        let (condition, inner) = self.condition(text, &["while"], &["is"], line, column)?;
        let body = self.block(&[Kind::EndWhile]);

        let outer = match self.next() {
            Some(end) => strip_any(&end.text, &["endwhile", "end"])
                .and_then(|rest| strip_word(rest.trim_start(), "while").or(Some(rest)))
                .and_then(paren)
                .map(|(label, _)| label)
                .filter(|l| !l.is_empty()),
            None => {
                self.error(
                    "unterminatedWhile",
                    "Falta el `endwhile` que cierra el `while`.",
                    line,
                    column,
                );
                None
            }
        };

        Some(Stmt::While(WhileLoop {
            condition,
            inner,
            outer,
            body,
        }))
    }

    fn repeat_loop(&mut self, line: u32, column: u32) -> Option<Stmt> {
        let body = self.block(&[Kind::RepeatWhile]);

        let Some(end) = self.next() else {
            self.error(
                "unterminatedRepeat",
                "Falta el `repeat while` que cierra el `repeat`.",
                line,
                column,
            );
            return None;
        };

        let (text, number, column) = (end.text.clone(), end.line, end.column);
        let (condition, inner) =
            self.condition(&text, &["repeat"], &["while", "is", "not"], number, column)?;

        Some(Stmt::Repeat(RepeatLoop {
            body,
            condition,
            inner,
            outer: None,
        }))
    }

    fn parallel(&mut self, opener: Kind, line: u32, column: u32) -> Option<Stmt> {
        let bars = opener == Kind::Fork;
        let (again, closers): (Kind, &[Kind]) = if bars {
            (
                Kind::ForkAgain,
                &[Kind::ForkAgain, Kind::EndFork, Kind::EndMerge],
            )
        } else {
            (Kind::SplitAgain, &[Kind::SplitAgain, Kind::EndSplit])
        };

        let mut branches = vec![self.block(closers)];
        let mut joins = true;

        loop {
            let Some(next) = self.peek() else {
                self.error(
                    "unterminatedFork",
                    "Falta el cierre del bloque de ramas paralelas.",
                    line,
                    column,
                );
                break;
            };

            let kind = kind_of(&next.text);
            self.next();

            if kind == again {
                branches.push(self.block(closers));
                continue;
            }

            // `end merge` junta los caminos sin barra de sincronizacion.
            joins = kind != Kind::EndMerge;
            break;
        }

        Some(Stmt::Parallel(Parallel {
            branches,
            bars,
            joins,
        }))
    }

    fn partition(&mut self, text: &str, line: u32, column: u32) -> Option<Stmt> {
        let rest = strip_any(text, &["partition", "group"])?;
        let rest = rest.trim();
        let rest = rest.strip_suffix('{').unwrap_or(rest).trim();

        let name = read_token(rest)
            .map(|(name, _)| name)
            .unwrap_or_else(|| rest.to_string());

        let body = self.block(&[Kind::Close]);

        if self.next().is_none() {
            self.error(
                "unterminatedPartition",
                "Falta la `}` que cierra la particion.",
                line,
                column,
            );
        }

        Some(Stmt::Partition { name, body })
    }
}

/// Texto de una accion, sin los dos puntos de delante ni el signo de detras.
fn action_text(text: &str) -> String {
    let text = text.strip_prefix(':').unwrap_or(text);
    let text = text.strip_suffix(ACTION_END).unwrap_or(text);

    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn ignored(flat: &str) -> Option<(&'static str, &'static str)> {
    if flat.starts_with('!') {
        return Some((
            "ignoredPreprocessor",
            "Las directivas de preprocesador no se ejecutan; lo que incluyan no entra.",
        ));
    }

    for word in [
        "note",
        "hide",
        "show",
        "skinparam",
        "scale",
        "header",
        "footer",
        "caption",
        "legend",
        "floating",
        "sprite",
        "style",
    ] {
        if flat == word || flat.starts_with(&format!("{word} ")) {
            return Some(("ignoredDirective", "La directiva se importa sin efecto."));
        }
    }

    None
}

/* --- Construccion del grafo ------------------------------------------------ */

/// Un extremo del que todavia tiene que salir una flecha.
#[derive(Clone)]
struct Open {
    node: String,
    label: Option<String>,
}

impl Open {
    fn at(node: impl Into<String>) -> Self {
        Open {
            node: node.into(),
            label: None,
        }
    }

    fn labelled(node: impl Into<String>, label: Option<String>) -> Self {
        Open {
            node: node.into(),
            label,
        }
    }
}

struct Builder {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    counter: usize,
    /// Particion abierta, si el recorrido esta dentro de una.
    partition: Option<String>,
}

impl Builder {
    fn new(now_ms: i64, diagnostics: Vec<Diagnostic>) -> Self {
        Builder {
            document: Document::new(
                uuid::Uuid::new_v4().to_string(),
                DiagramKind::Activity,
                now_ms,
            ),
            diagnostics,
            counter: 0,
            partition: None,
        }
    }

    fn finish(mut self) -> ImportResult {
        // Una particion sin nada dentro no tiene que dibujarse.
        let used: Vec<String> = self
            .document
            .nodes
            .iter()
            .filter_map(|n| n.parent.clone())
            .collect();

        self.document
            .nodes
            .retain(|n| n.shape != NodeShape::Package || used.iter().any(|p| p == &n.id));

        ImportResult::new(self.document, self.diagnostics)
    }

    fn node(&mut self, shape: NodeShape, label: &str) -> String {
        self.counter += 1;
        let id = format!("a{}", self.counter);

        let mut node = Node::new(&id, label, shape);
        node.parent = self.partition.clone();
        self.document.nodes.push(node);

        id
    }

    /// Cierra los extremos sueltos contra un nodo.
    fn link(&mut self, open: Vec<Open>, to: &str) {
        for end in open {
            self.counter += 1;

            let mut edge = Edge::new(
                format!("f{}", self.counter),
                end.node,
                to.to_string(),
                EdgeKind::Directed,
            );
            edge.routing = Routing::Orthogonal;
            edge.label = end.label;

            self.document.edges.push(edge);
        }
    }

    fn walk(&mut self, stmts: &[Stmt], mut open: Vec<Open>) -> Vec<Open> {
        for stmt in stmts {
            open = self.statement(stmt, open);
        }
        open
    }

    fn statement(&mut self, stmt: &Stmt, open: Vec<Open>) -> Vec<Open> {
        match stmt {
            Stmt::Action(text) => {
                let id = self.node(NodeShape::RoundedRect, text);
                self.link(open, &id);
                vec![Open::at(id)]
            }
            Stmt::Start => {
                let id = self.node(NodeShape::InitialNode, "");
                self.link(open, &id);
                vec![Open::at(id)]
            }
            Stmt::Stop => {
                let id = self.node(NodeShape::FinalNode, "");
                self.link(open, &id);
                // Un final no deja por donde seguir.
                Vec::new()
            }
            Stmt::If(conditional) => self.conditional(conditional, open),
            Stmt::While(loop_) => self.while_loop(loop_, open),
            Stmt::Repeat(loop_) => self.repeat_loop(loop_, open),
            Stmt::Parallel(parallel) => self.parallel(parallel, open),
            Stmt::Partition { name, body } => self.partition(name, body, open),
        }
    }

    /// Cada condicion es un rombo. El `elseif` cuelga de la salida que la
    /// condicion anterior no tomo, que es justo lo que significa.
    fn conditional(&mut self, conditional: &Conditional, open: Vec<Open>) -> Vec<Open> {
        let mut pending = open;
        let mut ends = Vec::new();

        for branch in &conditional.branches {
            let diamond = self.node(NodeShape::Diamond, &branch.condition);
            self.link(pending, &diamond);

            let entry = vec![Open::labelled(&diamond, branch.label.clone())];
            ends.extend(self.walk(&branch.body, entry));

            pending = vec![Open::at(diamond)];
        }

        match &conditional.otherwise {
            Some((label, body)) => {
                let entry = pending
                    .into_iter()
                    .map(|end| Open::labelled(end.node, label.clone()))
                    .collect();
                ends.extend(self.walk(body, entry));
            }
            // Sin `else`, la salida que no se toma es el camino que sigue.
            None => ends.extend(pending),
        }

        ends
    }

    fn while_loop(&mut self, loop_: &WhileLoop, open: Vec<Open>) -> Vec<Open> {
        let diamond = self.node(NodeShape::Diamond, &loop_.condition);
        self.link(open, &diamond);

        let entry = vec![Open::labelled(&diamond, loop_.inner.clone())];
        let ends = self.walk(&loop_.body, entry);

        // El cuerpo vuelve a la condicion: ahi esta el bucle.
        self.link(ends, &diamond);

        vec![Open::labelled(diamond, loop_.outer.clone())]
    }

    /// `repeat` lleva la condicion detras, asi que el cuerpo corre al menos
    /// una vez y la vuelta atras apunta a su primer nodo, no al rombo.
    fn repeat_loop(&mut self, loop_: &RepeatLoop, open: Vec<Open>) -> Vec<Open> {
        let before = self.document.nodes.len();
        let ends = self.walk(&loop_.body, open);
        let entry = self.document.nodes.get(before).map(|n| n.id.clone());

        let diamond = self.node(NodeShape::Diamond, &loop_.condition);
        self.link(ends, &diamond);

        if let Some(entry) = entry {
            let back = vec![Open::labelled(&diamond, loop_.inner.clone())];
            self.link(back, &entry);
        }

        vec![Open::labelled(diamond, loop_.outer.clone())]
    }

    fn parallel(&mut self, parallel: &Parallel, open: Vec<Open>) -> Vec<Open> {
        let entry = if parallel.bars {
            let bar = self.node(NodeShape::Bar, "");
            self.link(open, &bar);
            vec![Open::at(bar)]
        } else {
            open
        };

        let mut ends = Vec::new();
        for branch in &parallel.branches {
            ends.extend(self.walk(branch, entry.clone()));
        }

        if parallel.bars && parallel.joins {
            let bar = self.node(NodeShape::Bar, "");
            self.link(ends, &bar);
            return vec![Open::at(bar)];
        }

        ends
    }

    fn partition(&mut self, name: &str, body: &[Stmt], open: Vec<Open>) -> Vec<Open> {
        // La caja de la particion cuelga de donde estuviera el recorrido, no
        // de si misma.
        let id = self.node(NodeShape::Package, name);
        let outer = self.partition.replace(id);

        let ends = self.walk(body, open);

        self.partition = outer;
        ends
    }
}

#[cfg(test)]
mod tests;

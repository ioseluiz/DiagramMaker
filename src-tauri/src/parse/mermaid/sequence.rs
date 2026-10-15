//! Importador de `sequenceDiagram` de Mermaid.
//!
//! Tambien es un lenguaje de linea, pero a diferencia del `flowchart` lo que
//! construye es un arbol: `alt`, `loop` y `par` abren marcos que se cierran
//! con `end` y pueden anidarse. De ahi la pila de marcos abiertos, que es lo
//! unico realmente delicado de este modulo.
//!
//! Lo que no se entiende se reporta con linea y columna y se sigue leyendo.
//! Las directivas que Mermaid tiene y el documento no puede guardar
//! —`autonumber`, `box`, los enlaces de un participante— se avisan y se
//! saltan, nunca en silencio.

use std::collections::HashMap;

use crate::model::{
    Block, BlockKind, Branch, Diagnostic, DiagramKind, Document, Message, MessageKind, Node,
    NodeShape, NotePlacement, SequenceNote, SequenceScript, Span, Step,
};

use super::super::ImportResult;
use super::strip_comment;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Flechas, de mas larga a mas corta: `-->>` empieza como `-->`.
const ARROWS: &[(&str, MessageKind)] = &[
    ("<<-->>", MessageKind::Reply),
    ("<<->>", MessageKind::Sync),
    ("-->>", MessageKind::Reply),
    ("--)", MessageKind::Async),
    ("--x", MessageKind::Async),
    ("-->", MessageKind::Reply),
    ("->>", MessageKind::Sync),
    ("-)", MessageKind::Async),
    ("-x", MessageKind::Async),
    ("->", MessageKind::Sync),
];

/// Un marco a medio construir.
struct Frame {
    id: String,
    kind: BlockKind,
    /// Ramas ya cerradas por un `else` o un `and`.
    done: Vec<Branch>,
    /// Rama en curso.
    current: Branch,
    span: Span,
}

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre tal como aparece en el codigo -> id del nodo.
    by_name: HashMap<String, String>,
    /// Pasos del nivel raiz.
    root: Vec<Step>,
    /// Marcos abiertos, de fuera a dentro.
    frames: Vec<Frame>,
    /// `box` abiertos: agrupan participantes y su `end` no cierra un marco.
    boxes: usize,
    /// Participante declarado con `create`, a la espera de su mensaje.
    pending_create: Option<String>,
    /// Lo mismo con `destroy`.
    pending_destroy: Option<String>,
    counter: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        let mut document = Document::new(
            uuid::Uuid::new_v4().to_string(),
            DiagramKind::Sequence,
            now_ms,
        );
        document.script = Some(SequenceScript::default());

        Parser {
            document,
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            root: Vec::new(),
            frames: Vec::new(),
            boxes: 0,
            pending_create: None,
            pending_destroy: None,
            counter: 0,
        }
    }

    fn id(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}{}", self.counter)
    }

    fn error(&mut self, code: &str, message: impl Into<String>, line: u32, column: u32) {
        self.diagnostics
            .push(Diagnostic::error(code, message).at_span(Span::new(line, column)));
    }

    fn warn(&mut self, code: &str, message: impl Into<String>, line: u32, column: u32) {
        self.diagnostics
            .push(Diagnostic::warning(code, message).at_span(Span::new(line, column)));
    }

    fn run(&mut self, source: &str) {
        for (index, raw) in source.lines().enumerate() {
            let line = index as u32 + 1;
            let text = strip_comment(raw);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let column = (text.len() - text.trim_start().len()) as u32 + 1;
            self.line(trimmed, line, column);
        }

        for frame in std::mem::take(&mut self.frames) {
            self.diagnostics.push(
                Diagnostic::error("unterminatedBlock", "Falta el `end` que cierra el bloque.")
                    .at_span(frame.span),
            );
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower == "sequencediagram" {
            return;
        }

        if let Some(rest) = keyword(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        if lower == "end" {
            // Un `box` tambien se cierra con `end`, y no es un marco.
            if self.boxes > 0 {
                self.boxes -= 1;
            } else {
                self.close_frame(line, column);
            }
            return;
        }

        if keyword(&lower, "box").is_some() {
            self.boxes += 1;
            self.warn(
                "ignoredBox",
                "Un `box` agrupa participantes visualmente; el diagrama conserva su contenido pero no la caja.",
                line,
                column,
            );
            return;
        }

        // Marcos. `rect` tambien abre bloque: en Mermaid solo pinta un fondo,
        // pero agrupar es lo mas parecido que el documento sabe guardar.
        for (word, kind) in [
            ("alt", BlockKind::Alt),
            ("opt", BlockKind::Opt),
            ("loop", BlockKind::Loop),
            ("par", BlockKind::Par),
            ("critical", BlockKind::Critical),
            ("break", BlockKind::Break),
            ("rect", BlockKind::Group),
        ] {
            if let Some(rest) = keyword(text, word) {
                let label = if kind == BlockKind::Group {
                    String::new()
                } else {
                    rest.trim().to_string()
                };
                self.open_frame(kind, label, line, column);
                return;
            }
        }

        for word in ["else", "and", "option"] {
            if let Some(rest) = keyword(text, word) {
                self.divide(rest.trim().to_string(), line, column);
                return;
            }
        }

        if let Some(rest) = keyword(text, "create") {
            self.declare(rest.trim(), line, column);
            return;
        }

        for word in ["participant", "actor"] {
            if let Some(rest) = keyword(text, word) {
                let shape = if word == "actor" {
                    NodeShape::Actor
                } else {
                    NodeShape::Rectangle
                };
                self.participant(rest.trim(), shape, line, column);
                return;
            }
        }

        if let Some(rest) = keyword(text, "destroy") {
            match self.by_name.get(rest.trim()) {
                Some(id) => self.pending_destroy = Some(id.clone()),
                None => self.warn(
                    "unknownParticipant",
                    format!("`destroy` menciona `{}`, que no existe.", rest.trim()),
                    line,
                    column,
                ),
            }
            return;
        }

        for word in ["activate", "deactivate"] {
            if let Some(rest) = keyword(text, word) {
                self.activation(rest.trim(), word == "activate", line, column);
                return;
            }
        }

        if keyword(&lower, "note").is_some() {
            self.note(text, line, column);
            return;
        }

        for word in [
            "autonumber",
            "link",
            "links",
            "properties",
            "acctitle",
            "accdescr",
        ] {
            if keyword(&lower, word).is_some() {
                self.warn(
                    "ignoredDirective",
                    format!("`{word}` se importa sin efecto."),
                    line,
                    column,
                );
                return;
            }
        }

        self.message(text, line, column);
    }

    /* --- Participantes --------------------------------------------------- */

    /// `participant api as API REST`, o `participant api` a secas.
    fn participant(&mut self, rest: &str, shape: NodeShape, line: u32, column: u32) -> String {
        let (name, label) = match split_alias(rest) {
            Some((name, label)) => (name, label),
            None => (rest.to_string(), rest.to_string()),
        };

        if name.is_empty() {
            self.error(
                "missingParticipant",
                "Falta el nombre del participante.",
                line,
                column,
            );
            return String::new();
        }

        if let Some(id) = self.by_name.get(&name) {
            // Volver a declararlo le cambia la etiqueta y la forma.
            let id = id.clone();
            if let Some(node) = self.document.node_mut(&id) {
                node.label = label;
                node.shape = shape;
            }
            return id;
        }

        self.counter += 1;
        let id = format!("p{}_{}", self.counter, slug(&name));
        self.document.nodes.push(Node::new(&id, label, shape));
        self.by_name.insert(name, id.clone());

        id
    }

    /// `create participant b as B` / `create actor b`.
    fn declare(&mut self, rest: &str, line: u32, column: u32) {
        let (shape, body) = match (keyword(rest, "participant"), keyword(rest, "actor")) {
            (Some(body), _) => (NodeShape::Rectangle, body.trim()),
            (_, Some(body)) => (NodeShape::Actor, body.trim()),
            // `create b` sin palabra intermedia tambien se ve.
            _ => (NodeShape::Rectangle, rest),
        };

        let id = self.participant(body, shape, line, column);
        if !id.is_empty() {
            self.pending_create = Some(id);
        }
    }

    /// Nodo al que se refiere un nombre, creandolo si es la primera vez.
    ///
    /// En Mermaid no hace falta declarar un participante: mencionarlo en un
    /// mensaje basta, y el orden de aparicion es el orden del diagrama.
    fn node_for(&mut self, name: &str) -> String {
        if let Some(id) = self.by_name.get(name) {
            return id.clone();
        }

        self.counter += 1;
        let id = format!("p{}_{}", self.counter, slug(name));
        self.document
            .nodes
            .push(Node::new(&id, name, NodeShape::Rectangle));
        self.by_name.insert(name.to_string(), id.clone());

        id
    }

    /// `activate X` / `deactivate X` sueltos.
    ///
    /// Mermaid los escribe en su propia linea, detras del mensaje al que se
    /// refieren; el documento los guarda **en** ese mensaje. Se busca hacia
    /// atras el ultimo que encaje.
    fn activation(&mut self, name: &str, opening: bool, line: u32, column: u32) {
        let Some(id) = self.by_name.get(name).cloned() else {
            self.warn(
                "unknownParticipant",
                format!("`{name}` no existe todavia."),
                line,
                column,
            );
            return;
        };

        let found = last_message(self.steps_mut(), |m| {
            if opening { m.to == id } else { m.from == id }
        });

        match found {
            Some(message) => {
                if opening {
                    message.activate = true;
                } else {
                    message.deactivate = true;
                }
            }
            None => self.warn(
                "danglingActivation",
                format!("No hay ningun mensaje al que enganchar la activacion de `{name}`."),
                line,
                column,
            ),
        }
    }

    /* --- Mensajes -------------------------------------------------------- */

    fn message(&mut self, text: &str, line: u32, column: u32) {
        // El texto va detras de los primeros dos puntos y llega hasta el
        // final de la linea; uno dentro del texto no parte nada.
        let (head, body) = match text.split_once(':') {
            Some((head, body)) => (head, body.trim()),
            None => (text, ""),
        };

        let Some((at, arrow, kind)) = find_arrow(head) else {
            self.error(
                "badStatement",
                format!("No se entiende `{text}`."),
                line,
                column,
            );
            return;
        };

        let left = head[..at].trim();
        let mut right = head[at + arrow.len()..].trim();

        // `A->>+B` abre la ejecucion de B; `B-->>-A` cierra la de B.
        let mut activate = false;
        let mut deactivate = false;
        if let Some(rest) = right.strip_prefix('+') {
            activate = true;
            right = rest.trim();
        } else if let Some(rest) = right.strip_prefix('-') {
            deactivate = true;
            right = rest.trim();
        }

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la flecha le falta el participante de un lado.",
                line,
                column,
            );
            return;
        }

        let from = self.node_for(left);
        let to = self.node_for(right);

        // `create` y `destroy` marcan el mensaje que los cumple.
        let kind = if self.pending_create.as_deref() == Some(to.as_str()) {
            self.pending_create = None;
            MessageKind::Create
        } else if self.pending_destroy.as_deref() == Some(to.as_str())
            || self.pending_destroy.as_deref() == Some(from.as_str())
        {
            self.pending_destroy = None;
            MessageKind::Destroy
        } else {
            kind
        };

        let id = self.id("m");
        let mut message = Message::new(id, from, to, kind).with_text(clean(body));
        message.activate = activate;
        message.deactivate = deactivate;

        self.push(Step::Message(message));
    }

    /* --- Notas ----------------------------------------------------------- */

    fn note(&mut self, text: &str, line: u32, column: u32) {
        let Some((head, body)) = text.split_once(':') else {
            self.warn(
                "ignoredNote",
                "Una nota sin dos puntos no lleva texto; se descarta.",
                line,
                column,
            );
            return;
        };

        let words: Vec<&str> = head.split_whitespace().collect();
        let lower = head.trim().to_lowercase();

        let placement = if lower.starts_with("note over") {
            NotePlacement::Over
        } else if lower.starts_with("note left of") {
            NotePlacement::LeftOf
        } else if lower.starts_with("note right of") {
            NotePlacement::RightOf
        } else {
            self.error(
                "badNote",
                "Una nota se coloca con `over`, `left of` o `right of`.",
                line,
                column,
            );
            return;
        };

        // Los nombres van detras de la colocacion y se separan con comas.
        // Se leen del texto original: distinguen mayusculas.
        let named: Vec<String> = words[words_before_anchors(placement).min(words.len())..]
            .join(" ")
            .split(',')
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();

        let participants: Vec<String> = named
            .iter()
            .filter_map(|name| self.by_name.get(name).cloned())
            .collect();

        if participants.len() != named.len() {
            self.warn(
                "unknownParticipant",
                "La nota menciona un participante que no existe; se pega a los que si.",
                line,
                column,
            );
        }
        if participants.is_empty() {
            return;
        }

        let id = self.id("n");
        self.push(Step::Note(SequenceNote {
            id,
            placement,
            participants,
            text: clean(body.trim()),
        }));
    }

    /* --- Marcos ---------------------------------------------------------- */

    fn open_frame(&mut self, kind: BlockKind, label: String, line: u32, column: u32) {
        let id = self.id("b");
        let branch = self.id("r");

        self.frames.push(Frame {
            id,
            kind,
            done: Vec::new(),
            current: Branch::new(branch, label),
            span: Span::new(line, column),
        });
    }

    fn divide(&mut self, label: String, line: u32, column: u32) {
        let branch = self.id("r");

        let Some(frame) = self.frames.last_mut() else {
            self.error(
                "unexpectedElse",
                "`else` sin un bloque al que pertenecer.",
                line,
                column,
            );
            return;
        };

        let closed = std::mem::replace(&mut frame.current, Branch::new(branch, label));
        frame.done.push(closed);
    }

    fn close_frame(&mut self, line: u32, column: u32) {
        let Some(mut frame) = self.frames.pop() else {
            self.error(
                "unexpectedEnd",
                "`end` sin bloque que cerrar.",
                line,
                column,
            );
            return;
        };

        frame.done.push(frame.current);
        self.push(Step::Block(Block {
            id: frame.id,
            kind: frame.kind,
            branches: frame.done,
        }));
    }

    /// Lista de pasos donde cae lo siguiente: la rama del marco mas interno,
    /// o la raiz si no hay ninguno abierto.
    fn steps_mut(&mut self) -> &mut Vec<Step> {
        match self.frames.last_mut() {
            Some(frame) => &mut frame.current.steps,
            None => &mut self.root,
        }
    }

    fn push(&mut self, step: Step) {
        self.steps_mut().push(step);
    }

    fn finish(mut self) -> ImportResult {
        if self.pending_create.take().is_some() {
            self.diagnostics.push(Diagnostic::warning(
                "danglingCreate",
                "Un `create` se quedo sin el mensaje que crea al participante.",
            ));
        }

        let steps = std::mem::take(&mut self.root);
        self.document.script = Some(SequenceScript { steps });

        ImportResult::new(self.document, self.diagnostics)
    }
}

/* --- Ayudantes ----------------------------------------------------------- */

/// Cuantas palabras ocupa la colocacion antes de los nombres.
fn words_before_anchors(placement: NotePlacement) -> usize {
    match placement {
        // `Note over` son dos palabras; `Note left of`, tres.
        NotePlacement::Over => 2,
        _ => 3,
    }
}

/// Busca la flecha mas larga de la linea. Devuelve donde empieza y cual es.
fn find_arrow(text: &str) -> Option<(usize, &'static str, MessageKind)> {
    let bytes = text.as_bytes();

    for at in 0..bytes.len() {
        if !matches!(bytes[at], b'-' | b'<') {
            continue;
        }

        if let Some((pattern, kind)) = ARROWS
            .iter()
            .filter(|(pattern, _)| text[at..].starts_with(pattern))
            .max_by_key(|(pattern, _)| pattern.len())
        {
            return Some((at, pattern, *kind));
        }
    }

    None
}

/// Separa `api as API REST` en su nombre y su etiqueta.
fn split_alias(text: &str) -> Option<(String, String)> {
    let lower = text.to_lowercase();
    let at = lower.find(" as ")?;

    Some((
        text[..at].trim().to_string(),
        text[at + 4..].trim().to_string(),
    ))
}

/// Consume una palabra clave al principio, sin distinguir mayusculas.
fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    if text.len() < word.len() || !text[..word.len()].eq_ignore_ascii_case(word) {
        return None;
    }

    let rest = &text[word.len()..];
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some(rest)
}

/// Quita los saltos de linea de HTML, que es como Mermaid los escribe.
fn clean(text: &str) -> String {
    text.replace("<br/>", "\n").replace("<br>", "\n")
}

/// Ultimo mensaje del arbol que cumpla una condicion, en orden de lectura.
fn last_message(
    steps: &mut [Step],
    matches: impl Fn(&Message) -> bool + Copy,
) -> Option<&mut Message> {
    for step in steps.iter_mut().rev() {
        match step {
            Step::Message(message) if matches(message) => return Some(message),
            Step::Block(block) => {
                for branch in block.branches.iter_mut().rev() {
                    if let Some(found) = last_message(&mut branch.steps, matches) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;

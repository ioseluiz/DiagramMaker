//! Importador de diagramas de secuencia de PlantUML.
//!
//! Igual que el de Mermaid, construye un arbol: `alt`, `loop` y `par` abren
//! marcos que se cierran con `end` y se anidan.
//!
//! Lo propio de PlantUML son los sufijos que van pegados al destinatario:
//! `A -> B ++` abre su ejecucion, `--` la cierra, `**` lo crea y `!!` lo
//! destruye. Son cuatro cosas distintas escritas en el mismo sitio, y
//! pueden combinarse (`B --> A --++`), asi que se leen en bucle hasta que no
//! quede ninguno.

use std::collections::HashMap;

use crate::model::{
    Block, BlockKind, Branch, Diagnostic, DiagramKind, Document, Message, MessageKind, Node,
    NodeShape, NotePlacement, SequenceNote, SequenceScript, Span, Step,
};

use super::super::ImportResult;
use super::elements::{read_token, strip_word};

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Flechas, de mas larga a mas corta.
///
/// El tercer campo dice si la punta mira al participante de la izquierda; en
/// el modelo la punta va siempre al destino, asi que esas se leen al reves.
const ARROWS: &[(&str, MessageKind, bool)] = &[
    ("<<-->>", MessageKind::Reply, true),
    ("<<->>", MessageKind::Sync, true),
    ("-->>", MessageKind::Reply, false),
    ("<<--", MessageKind::Reply, true),
    ("->>", MessageKind::Async, false),
    ("<<-", MessageKind::Async, true),
    ("-->", MessageKind::Reply, false),
    ("<--", MessageKind::Reply, true),
    ("->x", MessageKind::Async, false),
    ("->o", MessageKind::Async, false),
    ("x<-", MessageKind::Async, true),
    ("o<-", MessageKind::Async, true),
    ("->", MessageKind::Sync, false),
    ("<-", MessageKind::Sync, true),
];

/// Palabras con las que se declara un participante y la forma de cada una.
const KINDS: &[(&str, NodeShape)] = &[
    ("participant", NodeShape::Rectangle),
    ("actor", NodeShape::Actor),
    ("boundary", NodeShape::Rectangle),
    ("control", NodeShape::Ellipse),
    ("entity", NodeShape::Rectangle),
    ("database", NodeShape::Cylinder),
    ("collections", NodeShape::Rectangle),
    ("queue", NodeShape::Hexagon),
];

/// Marcos y la palabra que los abre.
const BLOCKS: &[(&str, BlockKind)] = &[
    ("alt", BlockKind::Alt),
    ("opt", BlockKind::Opt),
    ("loop", BlockKind::Loop),
    ("par", BlockKind::Par),
    ("critical", BlockKind::Critical),
    ("break", BlockKind::Break),
    ("group", BlockKind::Group),
];

/// Un marco a medio construir.
struct Frame {
    id: String,
    kind: BlockKind,
    done: Vec<Branch>,
    current: Branch,
    span: Span,
}

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    by_name: HashMap<String, String>,
    root: Vec<Step>,
    frames: Vec<Frame>,
    /// Participante declarado con `create`, a la espera de su mensaje.
    pending_create: Option<String>,
    /// Bloque de texto libre que se esta saltando, por su palabra de cierre.
    skip_until: Option<String>,
    in_comment: bool,
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
            pending_create: None,
            skip_until: None,
            in_comment: false,
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
            let text = raw.trim();
            let column = (raw.len() - raw.trim_start().len()) as u32 + 1;

            if self.in_comment {
                if text.contains("'/") {
                    self.in_comment = false;
                }
                continue;
            }
            if text.starts_with("/'") {
                self.in_comment = !text.contains("'/");
                continue;
            }
            if text.is_empty() || text.starts_with('\'') {
                continue;
            }

            if let Some(keyword) = &self.skip_until {
                let lower = text.to_lowercase();
                if lower == format!("end {keyword}") || lower == format!("end{keyword}") {
                    self.skip_until = None;
                }
                continue;
            }

            self.line(text, line, column);
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

        if lower.starts_with("@start") || lower.starts_with("@end") {
            return;
        }

        if let Some(rest) = strip_word(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        if lower == "end" {
            self.close_frame(line, column);
            return;
        }

        for (word, kind) in BLOCKS {
            if let Some(rest) = strip_word(text, word) {
                self.open_frame(*kind, rest.trim().to_string(), line, column);
                return;
            }
        }

        if let Some(rest) = strip_word(text, "else") {
            self.divide(rest.trim().to_string(), line, column);
            return;
        }

        if let Some(rest) = strip_word(text, "create") {
            self.create(rest.trim(), line, column);
            return;
        }

        for (word, shape) in KINDS {
            if let Some(rest) = strip_word(text, word) {
                self.participant(rest.trim(), *shape, line, column);
                return;
            }
        }

        for word in ["activate", "deactivate", "destroy"] {
            if let Some(rest) = strip_word(text, word) {
                self.lifeline(rest.trim(), word, line, column);
                return;
            }
        }

        if strip_word(&lower, "note").is_some() || strip_word(&lower, "hnote").is_some() {
            self.note(text, line, column);
            return;
        }

        if let Some(code) = ignored(&lower) {
            self.warn(code.0, code.1, line, column);
            return;
        }

        self.message(text, line, column);
    }

    /* --- Participantes --------------------------------------------------- */

    /// `participant "API de pedidos" as api`, o `participant api` a secas.
    fn participant(&mut self, rest: &str, shape: NodeShape, line: u32, column: u32) -> String {
        let Some((label, tail)) = read_token(rest) else {
            self.error(
                "missingParticipant",
                "Falta el nombre del participante.",
                line,
                column,
            );
            return String::new();
        };

        if label.is_empty() {
            self.error(
                "missingParticipant",
                "Falta el nombre del participante.",
                line,
                column,
            );
            return String::new();
        }

        // `as alias` le da el nombre con el que se le referencia; el color
        // del final se ignora, que en una secuencia apenas se usa.
        let name = match strip_word(tail.trim_start(), "as").and_then(read_token) {
            Some((alias, _)) if !alias.is_empty() => alias,
            _ => label.clone(),
        };

        if let Some(id) = self.by_name.get(&name).cloned() {
            if let Some(node) = self.document.node_mut(&id) {
                node.label = label;
                node.shape = shape;
            }
            return id;
        }

        self.counter += 1;
        let id = format!("p{}_{}", self.counter, slug(&name));
        self.document.nodes.push(Node::new(&id, &label, shape));
        self.by_name.insert(name, id.clone());
        // Tambien por su texto visible: mezclar las dos formas es corriente.
        self.by_name.entry(label).or_insert_with(|| id.clone());

        id
    }

    fn create(&mut self, rest: &str, line: u32, column: u32) {
        // `create participant b` o `create b`.
        let (shape, body) = KINDS
            .iter()
            .find_map(|(word, shape)| strip_word(rest, word).map(|body| (*shape, body.trim())))
            .unwrap_or((NodeShape::Rectangle, rest));

        let id = self.participant(body, shape, line, column);
        if !id.is_empty() {
            self.pending_create = Some(id);
        }
    }

    fn node_for(&mut self, name: &str) -> String {
        let name = name.trim().trim_matches('"').trim();

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

    /// `activate X`, `deactivate X` y `destroy X` sueltos.
    ///
    /// PlantUML los escribe detras del mensaje al que se refieren; el
    /// documento los guarda **en** ese mensaje, asi que se busca hacia atras
    /// el ultimo que encaje.
    fn lifeline(&mut self, name: &str, word: &str, line: u32, column: u32) {
        let Some(id) = self.by_name.get(name).cloned() else {
            self.warn(
                "unknownParticipant",
                format!("`{word}` menciona `{name}`, que no existe."),
                line,
                column,
            );
            return;
        };

        let opening = word == "activate";
        let found = last_message(self.steps_mut(), |m| {
            if opening {
                m.to == id
            } else {
                m.from == id || m.to == id
            }
        });

        let Some(message) = found else {
            self.warn(
                "danglingActivation",
                format!("No hay ningun mensaje al que enganchar el `{word}` de `{name}`."),
                line,
                column,
            );
            return;
        };

        match word {
            "activate" => message.activate = true,
            "deactivate" => message.deactivate = true,
            _ => message.kind = MessageKind::Destroy,
        }
    }

    /* --- Mensajes -------------------------------------------------------- */

    fn message(&mut self, text: &str, line: u32, column: u32) {
        let (head, body) = split_label(text);

        let Some((at, arrow, kind, reversed)) = find_arrow(head) else {
            self.error(
                "badStatement",
                format!("No se entiende `{text}`."),
                line,
                column,
            );
            return;
        };

        let left = head[..at].trim();
        let (right, marks) = read_marks(head[at + arrow.len()..].trim());

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la flecha le falta el participante de un lado.",
                line,
                column,
            );
            return;
        }

        let (from, to) = if reversed {
            (self.node_for(right), self.node_for(left))
        } else {
            (self.node_for(left), self.node_for(right))
        };

        // Los sufijos van pegados al destinatario escrito, que con una
        // flecha al reves es el origen del mensaje.
        let marked = if reversed { &from } else { &to };

        let kind = if marks.creates || self.pending_create.as_deref() == Some(marked.as_str()) {
            self.pending_create = None;
            MessageKind::Create
        } else if marks.destroys {
            MessageKind::Destroy
        } else {
            kind
        };

        let id = self.id("m");
        let mut message = Message::new(id, from, to, kind).with_text(body);
        // `++` abre la ejecucion del destinatario escrito y `--` cierra la
        // del otro extremo, que es como el documento guarda las dos cosas.
        message.activate = marks.activates;
        message.deactivate = marks.deactivates;

        self.push(Step::Message(message));
    }

    /* --- Notas ----------------------------------------------------------- */

    fn note(&mut self, text: &str, line: u32, column: u32) {
        let (head, body) = split_label(text);
        let lower = head.trim().to_lowercase();

        let placement = if lower.contains("over") {
            NotePlacement::Over
        } else if lower.contains("left") {
            NotePlacement::LeftOf
        } else if lower.contains("right") {
            NotePlacement::RightOf
        } else {
            self.warn(
                "ignoredNote",
                "Una nota se coloca con `over`, `left of` o `right of`.",
                line,
                column,
            );
            return;
        };

        // Los nombres van detras de la colocacion; lo de delante son
        // palabras clave.
        let names: Vec<String> = head
            .split_whitespace()
            .skip_while(|word| {
                let w = word.to_lowercase();
                matches!(
                    w.as_str(),
                    "note" | "hnote" | "rnote" | "over" | "left" | "right" | "of"
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split(',')
            .map(|name| name.trim().trim_matches('"').trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();

        let participants: Vec<String> = names
            .iter()
            .filter_map(|name| self.by_name.get(name).cloned())
            .collect();

        if participants.is_empty() {
            self.warn(
                "ignoredNote",
                "La nota no se pega a ningun participante conocido.",
                line,
                column,
            );
            return;
        }

        // Una nota sin dos puntos abre un bloque de texto libre que llega
        // hasta `end note`.
        if body.is_empty() {
            self.skip_until = Some("note".into());
            self.warn(
                "ignoredNote",
                "Una nota de varias lineas no se importa: su texto no tiene donde ir.",
                line,
                column,
            );
            return;
        }

        let id = self.id("n");
        self.push(Step::Note(SequenceNote {
            id,
            placement,
            participants,
            text: body,
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

/// Sufijos que PlantUML pega al destinatario de un mensaje.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Marks {
    activates: bool,
    deactivates: bool,
    creates: bool,
    destroys: bool,
}

/// Separa los sufijos del nombre. Pueden venir varios: `B --++`.
fn read_marks(text: &str) -> (&str, Marks) {
    let mut rest = text.trim_end();
    let mut marks = Marks::default();

    loop {
        let trimmed = if let Some(head) = rest.strip_suffix("++") {
            marks.activates = true;
            head
        } else if let Some(head) = rest.strip_suffix("--") {
            marks.deactivates = true;
            head
        } else if let Some(head) = rest.strip_suffix("**") {
            marks.creates = true;
            head
        } else if let Some(head) = rest.strip_suffix("!!") {
            marks.destroys = true;
            head
        } else {
            break;
        };

        rest = trimmed.trim_end();
    }

    (rest.trim(), marks)
}

/// Parte la linea por los dos puntos que separan el texto del mensaje.
///
/// Los que van dentro del texto no cuentan, y un nombre entrecomillado puede
/// llevarlos: por eso se busca fuera de las comillas.
fn split_label(text: &str) -> (&str, String) {
    let bytes = text.as_bytes();
    let mut quoted = false;

    for index in 0..bytes.len() {
        match bytes[index] {
            b'"' => quoted = !quoted,
            b':' if !quoted => {
                return (
                    text[..index].trim(),
                    text[index + 1..].trim().replace("\\n", "\n"),
                );
            }
            _ => {}
        }
    }

    (text.trim(), String::new())
}

/// Busca la flecha de un mensaje. Devuelve donde empieza, cual es, que tipo
/// de mensaje dibuja y si mira al reves.
pub(crate) fn find_arrow(text: &str) -> Option<(usize, &'static str, MessageKind, bool)> {
    let bytes = text.as_bytes();

    for at in 0..bytes.len() {
        if !matches!(bytes[at], b'-' | b'<' | b'x' | b'o') {
            continue;
        }

        let best = ARROWS
            .iter()
            .filter(|(pattern, ..)| text[at..].starts_with(pattern))
            .max_by_key(|(pattern, ..)| pattern.len());

        if let Some((pattern, kind, reversed)) = best {
            // Una `x` o una `o` solo abren flecha si vienen despegadas: en
            // `box<-` la `x` es del nombre.
            if matches!(bytes[at], b'x' | b'o') && at > 0 && !bytes[at - 1].is_ascii_whitespace() {
                continue;
            }
            return Some((at, pattern, *kind, *reversed));
        }
    }

    None
}

/// Directivas que PlantUML entiende y el documento no necesita.
fn ignored(lower: &str) -> Option<(&'static str, &'static str)> {
    if lower.starts_with('!') {
        return Some((
            "ignoredPreprocessor",
            "Las directivas de preprocesador no se ejecutan; lo que incluyan no entra.",
        ));
    }
    if lower.starts_with("==") || lower == "..." || lower.starts_with("|||") {
        return Some((
            "ignoredDivider",
            "Los separadores y los saltos no se guardan en el guion.",
        ));
    }

    for word in [
        "autonumber",
        "skinparam",
        "hide",
        "show",
        "header",
        "footer",
        "caption",
        "newpage",
        "legend",
        "scale",
        "box",
        "end box",
        "ref",
        "return",
        "delay",
    ] {
        if strip_word(lower, word).is_some() {
            return Some(("ignoredDirective", "La directiva se importa sin efecto."));
        }
    }

    None
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

//! Importador de `stateDiagram-v2` de Mermaid.
//!
//! Lo unico realmente distinto de este dialecto es `[*]`. No es un estado:
//! es *el sitio por donde se entra o se sale*, y cual de las dos cosas sea
//! depende de a que lado de la flecha aparezca. A la izquierda es el disco de
//! arranque, a la derecha el anillo de salida.
//!
//! Ademas hay uno de cada por nivel de anidamiento, no uno por linea: dos
//! lineas que empiecen en `[*]` dentro del mismo estado compuesto salen del
//! mismo disco, que es como lo dibuja Mermaid.

use std::collections::HashMap;

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, LayoutDirection, Node, NodeShape, Routing,
    Span,
};

use super::super::{ImportResult, strip_word as keyword};
use super::strip_comment;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Lo que escribe Mermaid donde empieza o acaba la maquina.
const TERMINAL: &str = "[*]";

/// Estereotipos que cambian la forma del estado.
const STEREOTYPES: &[(&str, NodeShape)] = &[
    ("fork", NodeShape::Bar),
    ("join", NodeShape::Bar),
    ("choice", NodeShape::Diamond),
];

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre tal como aparece en el codigo -> indice en `document.nodes`.
    by_name: HashMap<String, usize>,
    /// Pila de estados compuestos abiertos.
    composites: Vec<String>,
    /// Disco de arranque de cada nivel, por el id de su contenedor.
    starts: HashMap<Option<String>, String>,
    /// Anillo de salida de cada nivel.
    ends: HashMap<Option<String>, String>,
    skipping_note: bool,
    counter: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        Parser {
            document: Document::new(uuid::Uuid::new_v4().to_string(), DiagramKind::State, now_ms),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            composites: Vec::new(),
            starts: HashMap::new(),
            ends: HashMap::new(),
            skipping_note: false,
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

            if self.skipping_note {
                if trimmed.eq_ignore_ascii_case("end note") {
                    self.skipping_note = false;
                }
                continue;
            }

            let column = (text.len() - text.trim_start().len()) as u32 + 1;
            self.line(trimmed, line, column);
        }

        for _ in std::mem::take(&mut self.composites) {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedState",
                "Falta la `}` que cierra el estado compuesto.",
            ));
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower.starts_with("statediagram") {
            return;
        }

        if text == "}" {
            if self.composites.pop().is_none() {
                self.error(
                    "unexpectedClose",
                    "`}` sin un estado compuesto que cerrar.",
                    line,
                    column,
                );
            }
            return;
        }

        if let Some(rest) = keyword(text, "direction") {
            if let Some(direction) = parse_direction(rest.trim()) {
                self.document.direction = direction;
            }
            return;
        }

        if let Some(rest) = keyword(text, "state") {
            self.declare(rest.trim(), line, column);
            return;
        }

        if let Some(rest) = keyword(&lower, "note") {
            // `note right of X : texto` cabe en su linea; sin los dos puntos,
            // abre un bloque que acaba en `end note`.
            if !rest.contains(':') {
                self.skipping_note = true;
            }
            self.warn(
                "ignoredNote",
                "Las notas no se importan: su texto no tiene donde ir en el diagrama.",
                line,
                column,
            );
            return;
        }

        for word in ["classdef", "class", "click", "style", "link"] {
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

        if find_arrow(text).is_some() {
            self.transition(text, line, column);
            return;
        }

        // `s1 : descripcion` le pone texto a un estado ya nombrado.
        if let Some((name, description)) = text.split_once(':') {
            let index = self.state_for(name.trim(), NodeShape::RoundedRect);
            self.document.nodes[index].label = description.trim().to_string();
            return;
        }

        // Un nombre suelto declara el estado y ya.
        if is_name(text) {
            self.state_for(text, NodeShape::RoundedRect);
            return;
        }

        self.error(
            "badStatement",
            format!("No se entiende `{text}`."),
            line,
            column,
        );
    }

    /* --- Estados ---------------------------------------------------------- */

    /// `state X`, `state "Texto" as X`, `state X <<fork>>`, `state X {`.
    fn declare(&mut self, rest: &str, line: u32, column: u32) {
        let opens = rest.ends_with('{');
        let head = rest.trim_end_matches('{').trim();

        // `state "Texto largo" as s1`
        if let Some(body) = head.strip_prefix('"') {
            let Some(end) = body.find('"') else {
                self.error(
                    "badState",
                    "Falta la comilla que cierra la descripcion.",
                    line,
                    column,
                );
                return;
            };
            let label = body[..end].trim().to_string();
            let name = keyword(body[end + 1..].trim(), "as")
                .map(str::trim)
                .unwrap_or("");

            if name.is_empty() {
                self.error(
                    "missingAlias",
                    "Una descripcion entre comillas necesita un `as` con su nombre.",
                    line,
                    column,
                );
                return;
            }

            let index = self.state_for(name, NodeShape::RoundedRect);
            self.document.nodes[index].label = label;
            if opens {
                let id = self.document.nodes[index].id.clone();
                self.composites.push(id);
            }
            return;
        }

        // `state s1 <<fork>>`
        let (name, shape) = match head.split_once("<<") {
            Some((name, tail)) => {
                let stereotype = tail.trim_end_matches(">>").trim().to_lowercase();
                let shape = STEREOTYPES
                    .iter()
                    .find(|(word, _)| *word == stereotype)
                    .map(|(_, shape)| *shape);

                if shape.is_none() {
                    self.warn(
                        "unknownStereotype",
                        format!("`<<{stereotype}>>` no cambia la forma; se dibuja como estado."),
                        line,
                        column,
                    );
                }
                (name.trim(), shape.unwrap_or(NodeShape::RoundedRect))
            }
            None => (head, NodeShape::RoundedRect),
        };

        if name.is_empty() {
            self.error("missingState", "Falta el nombre del estado.", line, column);
            return;
        }

        let index = self.state_for(name, shape);
        // Un estado que abre bloque es un contenedor, se haya declarado como
        // se haya declarado: dibujarlo como paquete es lo unico que deja ver
        // que hay cosas dentro.
        if opens {
            self.document.nodes[index].shape = NodeShape::Package;
            let id = self.document.nodes[index].id.clone();
            self.composites.push(id);
        }
    }

    /// Indice del estado, creandolo si es la primera vez que aparece.
    fn state_for(&mut self, name: &str, shape: NodeShape) -> usize {
        if let Some(&index) = self.by_name.get(name) {
            // Una segunda mencion con estereotipo le cambia la forma.
            if shape != NodeShape::RoundedRect {
                self.document.nodes[index].shape = shape;
            }
            return index;
        }

        self.counter += 1;
        let id = format!("s{}_{}", self.counter, slug(name));

        let mut node = Node::new(id, name, shape);
        node.parent = self.composites.last().cloned();

        let index = self.document.nodes.len();
        self.by_name.insert(name.to_string(), index);
        self.document.nodes.push(node);

        index
    }

    /// Indice del extremo que toca: el disco de arranque o el anillo de
    /// salida del nivel actual, creandolo la primera vez.
    fn terminal_for(&mut self, as_source: bool) -> usize {
        let parent = self.composites.last().cloned();

        let existing = if as_source {
            self.starts.get(&parent).cloned()
        } else {
            self.ends.get(&parent).cloned()
        };

        if let Some(id) = existing
            && let Some(index) = self.document.nodes.iter().position(|n| n.id == id)
        {
            return index;
        }

        self.counter += 1;
        let (id, shape) = if as_source {
            (format!("inicio{}", self.counter), NodeShape::InitialNode)
        } else {
            (format!("fin{}", self.counter), NodeShape::FinalNode)
        };

        let mut node = Node::new(&id, "", shape);
        node.parent = parent.clone();

        let index = self.document.nodes.len();
        self.document.nodes.push(node);

        if as_source {
            self.starts.insert(parent, id);
        } else {
            self.ends.insert(parent, id);
        }

        index
    }

    /// Resuelve un extremo de una transicion.
    fn endpoint_for(&mut self, name: &str, as_source: bool) -> usize {
        if name == TERMINAL {
            self.terminal_for(as_source)
        } else {
            self.state_for(name, NodeShape::RoundedRect)
        }
    }

    /* --- Transiciones ------------------------------------------------------ */

    fn transition(&mut self, text: &str, line: u32, column: u32) {
        let (head, label) = split_label(text);
        let Some((at, arrow)) = find_arrow(head) else {
            return;
        };

        let left = head[..at].trim();
        let right = head[at + arrow.len()..].trim();

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la transicion le falta el estado de un lado.",
                line,
                column,
            );
            return;
        }

        let from = self.endpoint_for(left, true);
        let to = self.endpoint_for(right, false);

        let id = self.id("t");
        let mut edge = Edge::new(
            id,
            self.document.nodes[from].id.clone(),
            self.document.nodes[to].id.clone(),
            EdgeKind::Directed,
        );
        edge.routing = Routing::Orthogonal;
        edge.label = label;

        self.document.edges.push(edge);
    }

    fn finish(mut self) -> ImportResult {
        // Un compuesto vacio no aporta nada y estorba al auto-layout.
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
}

/* --- Ayudantes ------------------------------------------------------------ */

/// Busca la flecha de una transicion.
///
/// Solo hay dos, y la larga se prueba primero por el mismo motivo de siempre.
fn find_arrow(text: &str) -> Option<(usize, &'static str)> {
    let bytes = text.as_bytes();

    for at in 0..bytes.len() {
        if bytes[at] != b'-' {
            continue;
        }
        for arrow in ["-->", "--"] {
            if text[at..].starts_with(arrow) {
                return Some((at, arrow));
            }
        }
    }

    None
}

/// Separa la etiqueta que va detras de los dos puntos.
fn split_label(text: &str) -> (&str, Option<String>) {
    match text.split_once(':') {
        Some((head, label)) => {
            let label = label.trim();
            (
                head.trim(),
                (!label.is_empty()).then(|| label.replace("<br/>", "\n")),
            )
        }
        None => (text.trim(), None),
    }
}

/// `true` si el texto puede ser el nombre de un estado.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn parse_direction(text: &str) -> Option<LayoutDirection> {
    match text.trim().to_uppercase().as_str() {
        "TD" | "TB" => Some(LayoutDirection::Down),
        "BT" => Some(LayoutDirection::Up),
        "LR" => Some(LayoutDirection::Right),
        "RL" => Some(LayoutDirection::Left),
        _ => None,
    }
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

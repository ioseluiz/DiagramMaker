//! Importador de maquinas de estados de PlantUML.
//!
//! Comparte con el de Mermaid lo que de verdad define el dialecto: `[*]` no
//! es un estado sino el sitio por donde se entra o se sale, y hay uno de cada
//! por nivel de anidamiento.
//!
//! Lo propio de PlantUML son tres cosas:
//!
//! - Un estado se describe con `Nuevo : esperando pago`, y las descripciones
//!   se acumulan: dos lineas sobre el mismo estado son dos renglones.
//! - Dentro de un compuesto, un `--` suelto separa regiones concurrentes. El
//!   documento no sabe guardarlas, asi que se avisa y lo de dentro se queda
//!   en el mismo contenedor.
//! - El reconocedor de flechas es el que ya existe para componentes: el
//!   vocabulario es el mismo, incluidas las pistas de direccion (`-down->`).

use std::collections::HashMap;

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, Node, NodeShape, Routing, Span,
};

use super::super::ImportResult;
use super::arrows;
use super::elements::{read_token, strip_word};

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Lo que escribe PlantUML donde empieza o acaba la maquina.
const TERMINAL: &str = "[*]";

/// Estereotipos que cambian la forma del estado.
const STEREOTYPES: &[(&str, NodeShape)] = &[
    ("fork", NodeShape::Bar),
    ("join", NodeShape::Bar),
    ("choice", NodeShape::Diamond),
    ("start", NodeShape::InitialNode),
    ("end", NodeShape::FinalNode),
];

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    by_name: HashMap<String, usize>,
    /// Pila de estados compuestos abiertos.
    composites: Vec<String>,
    starts: HashMap<Option<String>, String>,
    ends: HashMap<Option<String>, String>,
    skip_until: Option<String>,
    in_comment: bool,
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

        for _ in std::mem::take(&mut self.composites) {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedState",
                "Falta la `}` que cierra el estado compuesto.",
            ));
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower.starts_with("@start") || lower.starts_with("@end") {
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

        if let Some(rest) = strip_word(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        // Dentro de un compuesto, un `--` suelto separa regiones que corren a
        // la vez. El documento no sabe guardarlas.
        if !self.composites.is_empty() && text.chars().all(|c| c == '-' || c == '.' || c == '=') {
            self.warn(
                "ignoredRegion",
                "Las regiones concurrentes no se importan: lo de dentro queda en el mismo estado.",
                line,
                column,
            );
            return;
        }

        if let Some(rest) = strip_word(text, "state") {
            self.declare(rest.trim(), line, column);
            return;
        }

        if let Some(code) = ignored(&lower) {
            if strip_word(&lower, "note").is_some() && !text.contains(':') {
                self.skip_until = Some("note".into());
            }
            self.warn(code.0, code.1, line, column);
            return;
        }

        if arrows::find(text).is_some() {
            self.transition(text, line, column);
            return;
        }

        // `Nuevo : esperando pago` describe el estado. Las descripciones se
        // acumulan: PlantUML las apila en renglones.
        if let Some((name, description)) = text.split_once(':') {
            let name = name.trim();
            if name.is_empty() {
                self.error("badStatement", "Falta el estado.", line, column);
                return;
            }

            let index = self.state_for(name, NodeShape::RoundedRect);
            let node = &mut self.document.nodes[index];
            let extra = description.trim();
            if !extra.is_empty() {
                node.label = if node.label == name {
                    format!("{name}\n{extra}")
                } else {
                    format!("{}\n{extra}", node.label)
                };
            }
            return;
        }

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

        let Some((label, tail)) = read_token(head) else {
            self.error("missingState", "Falta el nombre del estado.", line, column);
            return;
        };
        if label.is_empty() {
            self.error("missingState", "Falta el nombre del estado.", line, column);
            return;
        }

        let mut name = label.clone();
        let mut shape = NodeShape::RoundedRect;
        let mut tail = tail;

        loop {
            let current = tail.trim_start();
            if current.is_empty() {
                break;
            }

            if let Some(after) = strip_word(current, "as") {
                let Some((alias, next)) = read_token(after) else {
                    break;
                };
                name = alias;
                tail = next;
                continue;
            }

            if let Some(after) = current.strip_prefix("<<") {
                let Some(end) = after.find(">>") else { break };
                let stereotype = after[..end].trim().to_lowercase();

                match STEREOTYPES.iter().find(|(word, _)| *word == stereotype) {
                    Some((_, found)) => shape = *found,
                    None => self.warn(
                        "unknownStereotype",
                        format!("`<<{stereotype}>>` no cambia la forma; se dibuja como estado."),
                        line,
                        column,
                    ),
                }
                tail = &after[end + 2..];
                continue;
            }

            break;
        }

        let index = self.state_for(&name, shape);
        self.document.nodes[index].label = label.clone();
        if label != name {
            self.by_name.entry(label).or_insert(index);
        }

        if opens {
            // Un estado que abre bloque es un contenedor: dibujarlo como
            // paquete es lo unico que deja ver que hay cosas dentro.
            self.document.nodes[index].shape = NodeShape::Package;
            let id = self.document.nodes[index].id.clone();
            self.composites.push(id);
        }
    }

    fn state_for(&mut self, name: &str, shape: NodeShape) -> usize {
        if let Some(&index) = self.by_name.get(name) {
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

    /// El arranque o la salida del nivel actual, creandolo la primera vez.
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

    fn endpoint_for(&mut self, name: &str, as_source: bool) -> usize {
        let name = unquote(name);
        if name == TERMINAL {
            self.terminal_for(as_source)
        } else {
            self.state_for(name, NodeShape::RoundedRect)
        }
    }

    /* --- Transiciones ------------------------------------------------------ */

    fn transition(&mut self, text: &str, line: u32, column: u32) {
        let (head, label) = split_label(text);
        let Some((at, end, arrow)) = arrows::find(head) else {
            return;
        };

        let left = head[..at].trim();
        let right = head[end..].trim();

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la transicion le falta el estado de un lado.",
                line,
                column,
            );
            return;
        }

        // En una maquina de estados toda transicion es dirigida; lo unico que
        // el reconocedor aporta aqui es si mira al reves.
        let (from, to) = if arrow.reversed {
            (
                self.endpoint_for(right, true),
                self.endpoint_for(left, false),
            )
        } else {
            (
                self.endpoint_for(left, true),
                self.endpoint_for(right, false),
            )
        };

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

/// Separa la etiqueta que va detras de los dos puntos, fuera de comillas.
fn split_label(text: &str) -> (&str, Option<String>) {
    let bytes = text.as_bytes();
    let mut quoted = false;

    for index in 0..bytes.len() {
        match bytes[index] {
            b'"' => quoted = !quoted,
            b':' if !quoted => {
                let label = text[index + 1..].trim();
                return (
                    text[..index].trim(),
                    (!label.is_empty()).then(|| label.replace("\\n", "\n")),
                );
            }
            _ => {}
        }
    }

    (text.trim(), None)
}

fn unquote(text: &str) -> &str {
    let text = text.trim();
    text.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text)
        .trim()
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn ignored(lower: &str) -> Option<(&'static str, &'static str)> {
    if lower.starts_with('!') {
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
        "left",
        "top",
    ] {
        if strip_word(lower, word).is_some() {
            return Some(("ignoredDirective", "La directiva se importa sin efecto."));
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

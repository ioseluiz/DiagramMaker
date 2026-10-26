//! Importador de `classDiagram` de Mermaid.
//!
//! Dos cosas lo separan de los otros dialectos:
//!
//! - Un miembro se escribe con el tipo **delante** del nombre
//!   (`+String nombre`), al reves que en PlantUML. Lo que distingue un
//!   atributo de una operacion son los parentesis, no la palabra.
//! - Lo estatico y lo abstracto van detras, pegados: `+metodo()$` es
//!   estatico y `+metodo()*` abstracto.
//!
//! Los miembros se pueden declarar dentro del bloque de la clase o sueltos
//! (`Pedido : +int id`), y las dos formas acaban en el mismo sitio.

use std::collections::HashMap;

use crate::model::{
    ClassBox, ClassMember, Diagnostic, DiagramKind, Document, Edge, EdgeKind, LayoutDirection,
    Node, NodeData, NodeShape, Routing, Span, Visibility,
};

use super::super::{ImportResult, strip_word as keyword};
use super::strip_comment;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Relaciones, de mas larga a mas corta.
///
/// El tercer campo dice si hay que leerla al reves: en el modelo la punta y
/// el rombo van siempre en el extremo que les toca —la punta al destino, el
/// rombo al origen— y Mermaid los escribe a los dos lados.
const ARROWS: &[(&str, EdgeKind, bool)] = &[
    ("<|--", EdgeKind::Inheritance, true),
    ("--|>", EdgeKind::Inheritance, false),
    ("<|..", EdgeKind::Realization, true),
    ("..|>", EdgeKind::Realization, false),
    ("*--", EdgeKind::Composition, false),
    ("--*", EdgeKind::Composition, true),
    ("o--", EdgeKind::Aggregation, false),
    ("--o", EdgeKind::Aggregation, true),
    ("-->", EdgeKind::Directed, false),
    ("<--", EdgeKind::Directed, true),
    ("..>", EdgeKind::Dependency, false),
    ("<..", EdgeKind::Dependency, true),
    ("--", EdgeKind::Association, false),
    ("..", EdgeKind::Association, false),
];

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre tal como aparece en el codigo -> indice en `document.nodes`.
    by_name: HashMap<String, usize>,
    /// Clase cuyo bloque esta abierto.
    open_class: Option<usize>,
    /// Pila de `namespace` abiertos, por el id de su paquete.
    namespaces: Vec<String>,
    counter: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        Parser {
            document: Document::new(uuid::Uuid::new_v4().to_string(), DiagramKind::Class, now_ms),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            open_class: None,
            namespaces: Vec::new(),
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

        if self.open_class.is_some() {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedClass",
                "Falta la `}` que cierra el bloque de la clase.",
            ));
        }
        for _ in std::mem::take(&mut self.namespaces) {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedNamespace",
                "Falta la `}` que cierra el `namespace`.",
            ));
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower.starts_with("classdiagram") {
            return;
        }

        if text == "}" {
            // El cierre vale para las dos cosas; la clase es la de dentro.
            if self.open_class.take().is_none() && self.namespaces.pop().is_none() {
                self.error(
                    "unexpectedClose",
                    "`}` sin un bloque que cerrar.",
                    line,
                    column,
                );
            }
            return;
        }

        // Dentro del bloque de una clase, todo lo que no sea el cierre es un
        // miembro suyo.
        if let Some(index) = self.open_class {
            self.member(index, text, line, column);
            return;
        }

        if let Some(rest) = keyword(text, "direction") {
            if let Some(direction) = parse_direction(rest.trim()) {
                self.document.direction = direction;
            }
            return;
        }

        if let Some(rest) = keyword(text, "namespace") {
            self.open_namespace(rest.trim(), line, column);
            return;
        }

        if let Some(rest) = keyword(text, "class") {
            self.declare(rest.trim(), line, column);
            return;
        }

        // `<<interface>> Pedido`: el estereotipo declarado fuera del bloque.
        if let Some(rest) = text.strip_prefix("<<") {
            self.stereotype(rest, line, column);
            return;
        }

        for word in ["note", "cssclass", "style", "click", "callback", "link"] {
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

        // Una relacion lleva flecha; lo que no la lleva y tiene dos puntos es
        // un miembro suelto: `Pedido : +int id`.
        if find_arrow(text).is_some() {
            self.relation(text, line, column);
            return;
        }

        if let Some((name, member)) = text.split_once(':') {
            let Some(index) = self.lookup(name.trim()) else {
                self.error(
                    "unknownClass",
                    format!("`{}` no es ninguna clase declarada.", name.trim()),
                    line,
                    column,
                );
                return;
            };
            self.member(index, member.trim(), line, column);
            return;
        }

        self.error(
            "badStatement",
            format!("No se entiende `{text}`."),
            line,
            column,
        );
    }

    /* --- Clases ---------------------------------------------------------- */

    /// `class Pedido`, `class Pedido {`, `class Pedido["Mi pedido"]`.
    fn declare(&mut self, rest: &str, line: u32, column: u32) {
        let opens = rest.ends_with('{');
        let head = rest.trim_end_matches('{').trim();

        // `Pedido:::estilo` aplica una clase CSS; el nombre es lo de delante.
        let head = head.split(":::").next().unwrap_or(head).trim();

        // `class Pedido["Etiqueta"]` cambia lo que se dibuja.
        let (name, label) = match head.split_once('[') {
            Some((name, rest)) => (
                name.trim(),
                rest.trim_end_matches(']')
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            ),
            None => (head, head.to_string()),
        };

        if name.is_empty() {
            self.error("missingClass", "Falta el nombre de la clase.", line, column);
            return;
        }

        let index = self.class_for(name);
        self.document.nodes[index].label = label;

        if opens {
            self.open_class = Some(index);
        }
    }

    /// `<<interface>> Pedido`
    fn stereotype(&mut self, rest: &str, line: u32, column: u32) {
        let Some((stereotype, name)) = rest.split_once(">>") else {
            self.error(
                "badStereotype",
                "Un estereotipo se cierra con `>>`.",
                line,
                column,
            );
            return;
        };

        let name = name.trim();
        let Some(index) = self.lookup(name) else {
            self.error(
                "unknownClass",
                format!("`{name}` no es ninguna clase declarada."),
                line,
                column,
            );
            return;
        };

        if let Some(box_) = self.document.nodes[index].data.as_class_mut() {
            box_.stereotype = Some(stereotype.trim().to_string());
        }
    }

    fn open_namespace(&mut self, rest: &str, line: u32, column: u32) {
        let name = rest.trim_end_matches('{').trim();
        if name.is_empty() {
            self.error(
                "missingNamespace",
                "Falta el nombre del `namespace`.",
                line,
                column,
            );
            return;
        }

        let id = self.id("ns");
        let mut node = Node::new(&id, name, NodeShape::Package);
        node.parent = self.namespaces.last().cloned();
        self.document.nodes.push(node);
        self.namespaces.push(id);
    }

    /// Indice de la clase, creandola si es la primera vez que aparece.
    fn class_for(&mut self, name: &str) -> usize {
        if let Some(&index) = self.by_name.get(name) {
            return index;
        }

        self.counter += 1;
        let id = format!("k{}_{}", self.counter, slug(name));

        let mut node = Node::new(id, name, NodeShape::Class);
        node.parent = self.namespaces.last().cloned();
        node.data = NodeData::Class(ClassBox::new());

        let index = self.document.nodes.len();
        self.by_name.insert(name.to_string(), index);
        self.document.nodes.push(node);

        index
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    /* --- Miembros --------------------------------------------------------- */

    fn member(&mut self, index: usize, text: &str, line: u32, column: u32) {
        // Un estereotipo tambien cabe dentro del bloque.
        if let Some(rest) = text.strip_prefix("<<") {
            if let Some((stereotype, _)) = rest.split_once(">>")
                && let Some(box_) = self.document.nodes[index].data.as_class_mut()
            {
                box_.stereotype = Some(stereotype.trim().to_string());
            }
            return;
        }

        let Some((member, is_operation)) = read_member(text) else {
            self.warn(
                "badMember",
                format!("No se entiende el miembro `{text}`."),
                line,
                column,
            );
            return;
        };

        let id = self.id(if is_operation { "o" } else { "a" });
        let member = ClassMember { id, ..member };

        if let Some(box_) = self.document.nodes[index].data.as_class_mut() {
            if is_operation {
                box_.operations.push(member);
            } else {
                box_.attributes.push(member);
            }
        }
    }

    /* --- Relaciones -------------------------------------------------------- */

    fn relation(&mut self, text: &str, line: u32, column: u32) {
        let (head, label) = split_label(text);
        let Some((at, arrow, kind, reversed)) = find_arrow(head) else {
            return;
        };

        let (left, left_multiplicity) = trailing_multiplicity(head[..at].trim());
        let (right, right_multiplicity) = leading_multiplicity(head[at + arrow.len()..].trim());

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la relacion le falta la clase de un lado.",
                line,
                column,
            );
            return;
        }

        let from = self.class_for(left);
        let to = self.class_for(right);

        let (source, target) = if reversed {
            (
                self.document.nodes[to].id.clone(),
                self.document.nodes[from].id.clone(),
            )
        } else {
            (
                self.document.nodes[from].id.clone(),
                self.document.nodes[to].id.clone(),
            )
        };
        let (source_multiplicity, target_multiplicity) = if reversed {
            (right_multiplicity, left_multiplicity)
        } else {
            (left_multiplicity, right_multiplicity)
        };

        let id = self.id("e");
        let mut edge = Edge::new(id, source, target, kind);
        edge.routing = Routing::Orthogonal;
        edge.label = label;
        edge.source.label = source_multiplicity;
        edge.target.label = target_multiplicity;

        // Un enlace punteado sin punta: el trazo es lo unico que lo dice.
        if arrow == ".." {
            edge.style.dashed = Some(true);
        }

        self.document.edges.push(edge);
    }

    fn finish(mut self) -> ImportResult {
        // Un `namespace` vacio no aporta nada y estorba al auto-layout.
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

/* --- Lectura de un miembro ----------------------------------------------- */

/// Lee `+String nombre` o `+metodo(int id) Pedido`.
///
/// Devuelve el miembro y si es una operacion. Lo que lo decide son los
/// parentesis: Mermaid no tiene palabra para distinguirlos.
pub(crate) fn read_member(text: &str) -> Option<(ClassMember, bool)> {
    let mut rest = text.trim();
    if rest.is_empty() {
        return None;
    }

    // Visibilidad, si la lleva.
    let mut visibility = Visibility::Public;
    if let Some(found) = rest.chars().next().and_then(Visibility::from_sigil) {
        visibility = found;
        rest = rest[1..].trim();
    }

    // `$` estatico y `*` abstracto, pegados al final.
    let mut is_static = false;
    let mut is_abstract = false;
    loop {
        let trimmed = if let Some(head) = rest.strip_suffix('$') {
            is_static = true;
            head
        } else if let Some(head) = rest.strip_suffix('*') {
            is_abstract = true;
            head
        } else {
            break;
        };
        rest = trimmed.trim_end();
    }

    if rest.is_empty() {
        return None;
    }

    let member = match rest.split_once('(') {
        // Operacion: `nombre(parametros) Retorno`.
        Some((name, tail)) => {
            let (parameters, after) = tail.split_once(')')?;
            ClassMember {
                id: String::new(),
                name: name.trim().to_string(),
                visibility,
                type_name: after.trim().to_string(),
                parameters: parameters.trim().to_string(),
                is_static,
                is_abstract,
            }
        }
        // Atributo: `Tipo nombre`, o solo el nombre.
        None => {
            let (type_name, name) = match rest.rsplit_once(char::is_whitespace) {
                Some((head, tail)) => (head.trim().to_string(), tail.trim().to_string()),
                None => (String::new(), rest.to_string()),
            };
            ClassMember {
                id: String::new(),
                name,
                visibility,
                type_name,
                parameters: String::new(),
                is_static,
                is_abstract,
            }
        }
    };

    if member.name.is_empty() {
        return None;
    }

    let is_operation = rest.contains('(');
    Some((member, is_operation))
}

/* --- Ayudantes ------------------------------------------------------------ */

/// Busca la relacion mas larga de la linea.
fn find_arrow(text: &str) -> Option<(usize, &'static str, EdgeKind, bool)> {
    let bytes = text.as_bytes();

    for at in 0..bytes.len() {
        if !matches!(bytes[at], b'-' | b'.' | b'<' | b'o' | b'*') {
            continue;
        }

        let best = ARROWS
            .iter()
            .filter(|(pattern, ..)| text[at..].starts_with(pattern))
            .max_by_key(|(pattern, ..)| pattern.len());

        if let Some((pattern, kind, reversed)) = best {
            // Una `o` o un `*` solo abren relacion despegados: en `banco--x`
            // la `o` es del nombre.
            if matches!(bytes[at], b'o' | b'*') && at > 0 && !bytes[at - 1].is_ascii_whitespace() {
                continue;
            }
            return Some((at, pattern, *kind, *reversed));
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

/// `Pedido "1"` -> (`Pedido`, `1`).
fn trailing_multiplicity(text: &str) -> (&str, Option<String>) {
    let text = text.trim();

    let Some(body) = text.strip_suffix('"') else {
        return (text, None);
    };
    let Some(open) = body.rfind('"') else {
        return (text, None);
    };

    let name = body[..open].trim();
    if name.is_empty() {
        return (text, None);
    }

    (name, Some(body[open + 1..].trim().to_string()))
}

/// `"0..*" Linea` -> (`Linea`, `0..*`).
fn leading_multiplicity(text: &str) -> (&str, Option<String>) {
    let text = text.trim();

    let Some(body) = text.strip_prefix('"') else {
        return (text, None);
    };
    let Some(close) = body.find('"') else {
        return (text, None);
    };

    let name = body[close + 1..].trim();
    if name.is_empty() {
        return (text, None);
    }

    (name, Some(body[..close].trim().to_string()))
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

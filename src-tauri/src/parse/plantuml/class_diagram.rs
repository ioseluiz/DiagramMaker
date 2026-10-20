//! Importador de diagramas de clases de PlantUML.
//!
//! Comparte el reconocedor de flechas con el de componentes: el vocabulario
//! de relaciones es el mismo —`<|--`, `*--`, `o--`, `..>`— y mantener dos
//! tablas seria pedir que divergieran.
//!
//! Lo propio de este dialecto son los miembros. PlantUML los escribe con el
//! tipo **detras** del nombre y separado por dos puntos (`- id : int`), al
//! reves que Mermaid, aunque tambien admite la forma de C (`-int id`); y los
//! modificadores van entre llaves (`{static}`, `{abstract}`), no pegados al
//! final.
//!
//! Lo que en Mermaid es un estereotipo aqui suele ser una palabra propia:
//! `interface Repositorio` en vez de `class Repositorio` con
//! `<<interface>>`. El documento lo guarda siempre como estereotipo, y el
//! emisor vuelve a escribirlo con su palabra.

use std::collections::HashMap;

use crate::model::{
    ClassBox, ClassMember, Diagnostic, DiagramKind, Document, Edge, Node, NodeData, NodeShape,
    Routing, Span, Visibility,
};

use super::super::ImportResult;
use super::arrows;
use super::component::{leading_multiplicity, trailing_multiplicity};
use super::elements::{read_token, strip_word};

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Palabras que declaran una clase, y el estereotipo que implica cada una.
///
/// `class` no implica ninguno; las demas sustituyen al `<<...>>` que
/// escribiria Mermaid.
const KEYWORDS: &[(&str, Option<&str>)] = &[
    ("abstract class", Some("abstract")),
    ("class", None),
    ("interface", Some("interface")),
    ("enum", Some("enumeration")),
    ("annotation", Some("annotation")),
    ("struct", Some("struct")),
    ("protocol", Some("protocol")),
    ("exception", Some("exception")),
    ("metaclass", Some("metaclass")),
    ("abstract", Some("abstract")),
];

/// Palabras que abren un contenedor.
const CONTAINERS: &[&str] = &["package", "namespace"];

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre tal como aparece en el codigo -> indice en `document.nodes`.
    by_name: HashMap<String, usize>,
    /// Clase cuyo bloque esta abierto.
    open_class: Option<usize>,
    /// Pila de contenedores abiertos. `None` para un `together`, que agrupa
    /// sin dibujar caja.
    containers: Vec<Option<String>>,
    skip_until: Option<String>,
    in_comment: bool,
    counter: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        Parser {
            document: Document::new(uuid::Uuid::new_v4().to_string(), DiagramKind::Class, now_ms),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            open_class: None,
            containers: Vec::new(),
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

        if self.open_class.is_some() {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedClass",
                "Falta la `}` que cierra el bloque de la clase.",
            ));
        }
        for _ in std::mem::take(&mut self.containers) {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedBlock",
                "Falta la `}` que cierra el bloque.",
            ));
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower.starts_with("@start") || lower.starts_with("@end") {
            return;
        }

        if text == "}" {
            // El cierre vale para las dos cosas; la clase es la de dentro.
            if self.open_class.take().is_none() && self.containers.pop().is_none() {
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

        if let Some(rest) = strip_word(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        if elements_together(&lower) {
            self.containers.push(None);
            return;
        }

        for word in CONTAINERS {
            if let Some(rest) = strip_word(text, word) {
                self.open_container(rest.trim(), line, column);
                return;
            }
        }

        // Las relaciones van antes que las declaraciones: `A <|-- B` empieza
        // por un nombre, no por una palabra clave, pero `class A` tambien
        // podria llevar una flecha en su estereotipo.
        if !starts_with_keyword(&lower) && arrows::find(text).is_some() {
            self.relation(text, line, column);
            return;
        }

        for (word, stereotype) in KEYWORDS {
            if let Some(rest) = strip_word(text, word) {
                self.declare(rest.trim(), *stereotype, line, column);
                return;
            }
        }

        if arrows::find(text).is_some() {
            self.relation(text, line, column);
            return;
        }

        if let Some(code) = ignored(&lower) {
            if strip_word(&lower, "note").is_some() && !text.contains(':') {
                self.skip_until = Some("note".into());
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

    /* --- Clases ---------------------------------------------------------- */

    /// `class Pedido`, `interface Repo {`, `class "Pedido de compra" as p`.
    fn declare(&mut self, rest: &str, implied: Option<&str>, line: u32, column: u32) {
        let opens = rest.ends_with('{');
        let head = rest.trim_end_matches('{').trim();

        let Some((label, mut tail)) = read_token(head) else {
            self.error("missingClass", "Falta el nombre de la clase.", line, column);
            return;
        };
        if label.is_empty() {
            self.error("missingClass", "Falta el nombre de la clase.", line, column);
            return;
        }

        // `as alias` y `<<estereotipo>>`, en cualquier orden.
        let mut name = label.clone();
        let mut stereotype = implied.map(str::to_string);

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
                let Some(end) = after.find(">>") else {
                    break;
                };
                stereotype = Some(after[..end].trim().to_string());
                tail = &after[end + 2..];
                continue;
            }

            break;
        }

        let index = self.class_for(&name);
        self.document.nodes[index].label = label.clone();
        // Tambien por su texto visible: mezclar las dos formas es corriente.
        if label != name {
            self.by_name.entry(label).or_insert(index);
        }

        if let Some(stereotype) = stereotype
            && let Some(box_) = self.document.nodes[index].data.as_class_mut()
        {
            box_.stereotype = Some(stereotype);
        }

        if opens {
            self.open_class = Some(index);
        }
    }

    fn open_container(&mut self, rest: &str, line: u32, column: u32) {
        let head = rest.trim_end_matches('{').trim();
        let name = read_token(head).map(|(name, _)| name).unwrap_or_default();

        if name.is_empty() {
            self.error(
                "missingPackage",
                "Falta el nombre del paquete.",
                line,
                column,
            );
            return;
        }

        let id = self.id("pk");
        let mut node = Node::new(&id, name, NodeShape::Package);
        node.parent = self.parent();
        self.document.nodes.push(node);
        self.containers.push(Some(id));
    }

    /// Contenedor actual: el bloque abierto mas reciente que dibuje caja.
    fn parent(&self) -> Option<String> {
        self.containers.iter().rev().find_map(Clone::clone)
    }

    /// Indice de la clase, creandola si es la primera vez que aparece.
    fn class_for(&mut self, name: &str) -> usize {
        if let Some(&index) = self.by_name.get(name) {
            return index;
        }

        self.counter += 1;
        let id = format!("k{}_{}", self.counter, slug(name));

        let mut node = Node::new(id, name, NodeShape::Class);
        node.parent = self.parent();
        node.data = NodeData::Class(ClassBox::new());

        let index = self.document.nodes.len();
        self.by_name.insert(name.to_string(), index);
        self.document.nodes.push(node);

        index
    }

    /* --- Miembros --------------------------------------------------------- */

    fn member(&mut self, index: usize, text: &str, line: u32, column: u32) {
        // `--` y `..` sueltos parten la caja en compartimentos; el documento
        // ya los tiene fijos y no hay donde guardarlos.
        if text
            .chars()
            .all(|c| c == '-' || c == '.' || c == '=' || c == '_')
        {
            return;
        }

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
        let Some((at, end, arrow)) = arrows::find(head) else {
            return;
        };

        let (left, left_multiplicity) = trailing_multiplicity(head[..at].trim());
        let (right, right_multiplicity) = leading_multiplicity(head[end..].trim());

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la relacion le falta la clase de un lado.",
                line,
                column,
            );
            return;
        }

        let from = self.class_for(unquote(left));
        let to = self.class_for(unquote(right));

        let (source, target) = if arrow.reversed {
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
        let (source_multiplicity, target_multiplicity) = if arrow.reversed {
            (right_multiplicity, left_multiplicity)
        } else {
            (left_multiplicity, right_multiplicity)
        };

        let id = self.id("e");
        let mut edge = Edge::new(id, source, target, arrow.kind);
        edge.routing = Routing::Orthogonal;
        edge.label = label;
        edge.source.label = source_multiplicity;
        edge.target.label = target_multiplicity;

        if arrow.dashed != arrow.kind.is_dashed_by_default() {
            edge.style.dashed = Some(arrow.dashed);
        }

        self.document.edges.push(edge);
    }

    fn finish(mut self) -> ImportResult {
        // Un paquete vacio no aporta nada y estorba al auto-layout.
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

/// Lee `- id : int`, `-int id` o `+ buscar(id : int) : Pedido`.
///
/// Devuelve el miembro y si es una operacion. Lo deciden los parentesis,
/// salvo que un `{field}` o un `{method}` lo digan explicitamente.
pub(crate) fn read_member(text: &str) -> Option<(ClassMember, bool)> {
    let mut rest = text.trim();
    if rest.is_empty() {
        return None;
    }

    let mut visibility = Visibility::Public;
    if let Some(found) = rest.chars().next().and_then(Visibility::from_sigil) {
        visibility = found;
        rest = rest[1..].trim();
    }

    // `{static}`, `{abstract}`, `{field}` y `{method}`, en cualquier orden.
    let mut is_static = false;
    let mut is_abstract = false;
    let mut forced: Option<bool> = None;

    while let Some(after) = rest.strip_prefix('{') {
        let Some(end) = after.find('}') else { break };
        match after[..end].trim().to_lowercase().as_str() {
            "static" => is_static = true,
            "abstract" => is_abstract = true,
            "field" => forced = Some(false),
            "method" => forced = Some(true),
            _ => {}
        }
        rest = after[end + 1..].trim();
    }

    if rest.is_empty() {
        return None;
    }

    let has_parens = rest.contains('(');
    let is_operation = forced.unwrap_or(has_parens);

    let member = if has_parens {
        let (name, tail) = rest.split_once('(')?;
        let (parameters, after) = tail.split_once(')')?;

        ClassMember {
            id: String::new(),
            name: name.trim().to_string(),
            visibility,
            // Lo que devuelve va detras de los dos puntos.
            type_name: after.trim().trim_start_matches(':').trim().to_string(),
            parameters: parameters.trim().to_string(),
            is_static,
            is_abstract,
        }
    } else {
        let (name, type_name) = match rest.split_once(':') {
            // `id : int`, la forma propia de PlantUML.
            Some((name, type_name)) => (name.trim().to_string(), type_name.trim().to_string()),
            // `int id`, la forma de C que tambien admite.
            None => match rest.split_once(char::is_whitespace) {
                Some((type_name, name)) => (name.trim().to_string(), type_name.trim().to_string()),
                None => (rest.to_string(), String::new()),
            },
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
    };

    if member.name.is_empty() {
        return None;
    }

    Some((member, is_operation))
}

/* --- Ayudantes ------------------------------------------------------------ */

/// `true` si la linea empieza por una palabra que declara una clase.
fn starts_with_keyword(lower: &str) -> bool {
    KEYWORDS
        .iter()
        .any(|(word, _)| strip_word(lower, word).is_some())
}

fn elements_together(lower: &str) -> bool {
    strip_word(lower, "together").is_some()
}

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

/// Directivas que PlantUML entiende y el documento no necesita.
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
        "set",
        "scale",
        "header",
        "footer",
        "caption",
        "legend",
        "remove",
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

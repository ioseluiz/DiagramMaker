//! Importador de diagramas de componentes de PlantUML.
//!
//! Como el de Mermaid, es un lector de lineas: cada una declara un elemento,
//! una relacion o una directiva. El subconjunto que entiende es el de un
//! diagrama de arquitectura real —componentes, paquetes anidados, bases de
//! datos, colas, actores y las relaciones entre ellos— y nada mas.
//!
//! Lo que queda fuera no se adivina: las notas, las leyendas y los
//! `skinparam` se reconocen, se avisan y se saltan enteros; lo que no se
//! reconoce se reporta con linea y columna. En ningun caso se interpreta a
//! ojo ni se descarta en silencio.
//!
//! No se intenta cubrir la libreria de macros C4 (`!include C4_Context.puml`,
//! `Container(...)`, `Rel(...)`): es un lenguaje propio por encima de
//! PlantUML y merece su propio importador.

use std::collections::HashMap;

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, LayoutDirection, Node, NodeShape, Routing, Span,
};

use super::super::ImportResult;
use super::{arrows, elements};

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    import_as(source, now_ms, DiagramKind::Architecture)
}

/// Lo mismo, pero diciendo de que tipo es el documento.
///
/// Un diagrama de casos de uso se lee con este mismo parser: estructuralmente
/// son lo mismo —formas con etiqueta, anidadas en contenedores y unidas por
/// relaciones con tipo— y lo unico que cambia es que palabras aparecen y que
/// tipo lleva el documento. Duplicar quinientas lineas para cambiar eso seria
/// pedir que las dos copias divergieran.
pub fn import_as(source: &str, now_ms: i64, kind: DiagramKind) -> ImportResult {
    let mut parser = Parser::new(now_ms, kind);
    parser.run(source);
    parser.finish()
}

/// Un bloque `{ ... }` abierto.
struct Open {
    /// Contenedor al que van los elementos de dentro. `None` para un
    /// `together`, que agrupa para el layout pero no dibuja caja.
    id: Option<String>,
    span: Span,
}

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre o alias tal como aparece en el codigo -> indice del nodo.
    by_name: HashMap<String, usize>,
    stack: Vec<Open>,
    /// Bloque que se esta saltando, por su palabra de cierre.
    skip_until: Option<String>,
    /// Llaves abiertas dentro de un bloque que se salta entero, como el
    /// cuerpo de un `skinparam`. Hay que contarlas para saber donde acaba.
    skip_depth: usize,
    in_comment: bool,
    started: bool,
    next_node: usize,
    next_edge: usize,
}

impl Parser {
    fn new(now_ms: i64, kind: DiagramKind) -> Self {
        Parser {
            document: Document::new(uuid::Uuid::new_v4().to_string(), kind, now_ms),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            stack: Vec::new(),
            skip_until: None,
            skip_depth: 0,
            in_comment: false,
            started: false,
            next_node: 0,
            next_edge: 0,
        }
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

            if self.skip_depth > 0 {
                if text.ends_with('{') {
                    self.skip_depth += 1;
                } else if text == "}" {
                    self.skip_depth -= 1;
                }
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

        for open in std::mem::take(&mut self.stack) {
            self.diagnostics.push(
                Diagnostic::error("unterminatedBlock", "Falta la `}` que cierra el bloque.")
                    .at_span(open.span),
            );
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if lower.starts_with("@start") {
            self.started = true;
            return;
        }
        if lower.starts_with("@end") {
            return;
        }

        if text == "}" {
            if self.stack.pop().is_none() {
                self.error(
                    "unexpectedClose",
                    "`}` sin un bloque que cerrar.",
                    line,
                    column,
                );
            }
            return;
        }

        if let Some(rest) = elements::strip_word(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        match lower.as_str() {
            "left to right direction" => {
                self.document.direction = LayoutDirection::Right;
                return;
            }
            "top to bottom direction" => {
                self.document.direction = LayoutDirection::Down;
                return;
            }
            _ => {}
        }

        // `together { ... }` agrupa para el layout pero no dibuja caja: sus
        // elementos cuelgan de donde colgaria el propio `together`.
        if elements::strip_word(&lower, "together").is_some() {
            self.stack.push(Open {
                id: None,
                span: Span::new(line, column),
            });
            return;
        }

        if let Some(skipped) = note_or_legend(&lower) {
            if let Skipped::Block(keyword) = skipped {
                self.skip_until = Some(keyword.to_string());
            }
            self.warn(
                "ignoredNote",
                "Las notas y las leyendas no se importan: su texto no tiene donde ir en el diagrama.",
                line,
                column,
            );
            return;
        }

        if let Some((code, message)) = ignored_directive(&lower) {
            // `skinparam x { ... }` lleva ajustes dentro, no elementos: el
            // bloque entero se salta.
            if text.ends_with('{') {
                self.skip_depth = 1;
            }
            self.warn(code, message, line, column);
            return;
        }

        // Las relaciones van antes que las declaraciones: `[A] --> [B]`
        // empieza igual que `[A]`.
        if let Some((start, end, arrow)) = arrows::find(text) {
            self.relation(text, start, end, arrow, line, column);
            return;
        }

        if let Some(declared) = elements::parse(text) {
            self.declare(declared, line, column);
            return;
        }

        self.error(
            "badStatement",
            format!("No se entiende `{text}`."),
            line,
            column,
        );
    }

    fn declare(&mut self, declared: elements::Declared, line: u32, column: u32) {
        if !declared.trailing.is_empty() {
            self.warn(
                "ignoredDeclarationTail",
                format!(
                    "`{}` sobra en la declaracion y se ignora.",
                    declared.trailing
                ),
                line,
                column,
            );
        }

        let index = match self.by_name.get(&declared.name).copied() {
            // Declarar un elemento despues de haberlo usado le pone forma y
            // etiqueta. Es corriente escribir las relaciones primero.
            Some(index) => {
                let node = &mut self.document.nodes[index];
                node.shape = declared.shape;
                node.label = declared.label.clone();
                index
            }
            None => self.create(declared.shape, &declared.name, &declared.label),
        };

        if let Some(fill) = &declared.fill {
            self.document.nodes[index].style.fill = Some(normalize_color(fill));
        }

        // Con `component "API REST" as api`, las relaciones pueden nombrarlo
        // por el alias o por el texto visible.
        if declared.label != declared.name {
            self.by_name.entry(declared.label.clone()).or_insert(index);
        }

        if declared.opens_block {
            let id = self.document.nodes[index].id.clone();
            self.stack.push(Open {
                id: Some(id),
                span: Span::new(line, column),
            });
        }
    }

    fn relation(
        &mut self,
        text: &str,
        start: usize,
        end: usize,
        arrow: arrows::Arrow,
        line: u32,
        column: u32,
    ) {
        let left = text[..start].trim();
        let (right, label) = split_label(text[end..].trim());

        if left.is_empty() || right.is_empty() {
            self.error(
                "danglingArrow",
                "A la flecha le falta el elemento de un lado.",
                line,
                column,
            );
            return;
        }

        let (left_name, left_multiplicity) = trailing_multiplicity(left);
        let (right_name, right_multiplicity) = leading_multiplicity(right);

        let from = self.element_for(left_name);
        let to = self.element_for(right_name);

        self.next_edge += 1;
        let mut source = self.document.nodes[from].id.clone();
        let mut target = self.document.nodes[to].id.clone();
        let (mut source_multiplicity, mut target_multiplicity) =
            (left_multiplicity, right_multiplicity);

        // En el modelo la punta va siempre al destino; si en el codigo miraba
        // al otro lado, la relacion es la misma leida al reves.
        if arrow.reversed {
            std::mem::swap(&mut source, &mut target);
            std::mem::swap(&mut source_multiplicity, &mut target_multiplicity);
        }

        let mut edge = Edge::new(format!("e{}", self.next_edge), source, target, arrow.kind);
        edge.routing = Routing::Orthogonal;
        edge.label = label;
        edge.source.label = source_multiplicity;
        edge.target.label = target_multiplicity;

        // Solo se guarda el trazo cuando no es el de por defecto de la
        // relacion: una dependencia ya se dibuja punteada por si misma.
        if arrow.dashed != arrow.kind.is_dashed_by_default() {
            edge.style.dashed = Some(arrow.dashed);
        }

        self.document.edges.push(edge);
    }

    /// Indice del elemento referenciado, creandolo si no existia.
    ///
    /// Usar un componente sin declararlo es normal en PlantUML: `[A] --> [B]`
    /// basta para que existan los dos.
    fn element_for(&mut self, reference: &str) -> usize {
        let name = elements::unwrap_reference(reference).to_string();

        if let Some(&index) = self.by_name.get(&name) {
            return index;
        }

        // La forma corta con la que se le menciona dice que es: `(Comprar)`
        // es un caso de uso y `:Cliente:` un actor, aunque nadie los haya
        // declarado antes.
        let shape = elements::shorthand_shape(reference).unwrap_or(NodeShape::Rectangle);
        self.create(shape, &name, &name)
    }

    fn create(&mut self, shape: NodeShape, name: &str, label: &str) -> usize {
        self.next_node += 1;

        // El contador delante del nombre evita que dos nombres distintos que
        // se limpian igual (`api-rest` y `api.rest`) acaben con el mismo id.
        let mut node = Node::new(format!("n{}_{}", self.next_node, slug(name)), label, shape);
        node.parent = self.parent();

        let index = self.document.nodes.len();
        self.by_name.insert(name.to_string(), index);
        self.document.nodes.push(node);

        index
    }

    /// Contenedor actual: el bloque abierto mas reciente que dibuje caja.
    fn parent(&self) -> Option<String> {
        self.stack.iter().rev().find_map(|open| open.id.clone())
    }

    fn finish(mut self) -> ImportResult {
        if !self.started && !self.document.nodes.is_empty() {
            self.diagnostics.push(
                Diagnostic::warning(
                    "missingHeader",
                    "Falta la linea `@startuml`; se asume un diagrama de componentes.",
                )
                .at_span(Span::new(1, 1)),
            );
        }

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

/// Cuanto ocupa una nota o una leyenda.
enum Skipped {
    /// Cabe en su linea: `note left of api : texto`.
    Line,
    /// Abre bloque y se cierra con `end <palabra>`.
    Block(&'static str),
}

/// Reconoce las notas y las leyendas, que llevan texto libre dentro y no
/// tienen donde ir en el documento.
fn note_or_legend(lower: &str) -> Option<Skipped> {
    for keyword in ["note", "legend"] {
        let Some(rest) = elements::strip_word(lower, keyword) else {
            continue;
        };

        // `note left of api : texto` y `note "texto" as n1` caben en una
        // linea; `note right of api` sin dos puntos abre bloque.
        return Some(if rest.contains(':') || rest.contains('"') {
            Skipped::Line
        } else {
            Skipped::Block(keyword)
        });
    }

    None
}

/// Directivas que PlantUML entiende y el documento no necesita.
fn ignored_directive(lower: &str) -> Option<(&'static str, &'static str)> {
    if lower.starts_with('!') {
        return Some((
            "ignoredPreprocessor",
            "Las directivas de preprocesador no se ejecutan; lo que incluyan no entra.",
        ));
    }

    for keyword in [
        "skinparam",
        "hide",
        "show",
        "scale",
        "caption",
        "header",
        "footer",
        "allow_mixing",
        "autonumber",
        "mainframe",
    ] {
        if elements::strip_word(lower, keyword).is_some() {
            return Some(("ignoredDirective", "La directiva se importa sin efecto."));
        }
    }

    None
}

/// Separa la etiqueta que va detras de los dos puntos.
fn split_label(text: &str) -> (&str, Option<String>) {
    let bytes = text.as_bytes();
    let mut quoted = false;

    for index in 0..bytes.len() {
        match bytes[index] {
            b'"' => quoted = !quoted,
            b':' if !quoted => {
                let label = text[index + 1..].trim();
                let label = (!label.is_empty()).then(|| label.replace("\\n", "\n"));
                return (text[..index].trim(), label);
            }
            _ => {}
        }
    }

    (text, None)
}

/// `api "1"` -> (`api`, `1`). La multiplicidad va pegada a la flecha.
pub(super) fn trailing_multiplicity(text: &str) -> (&str, Option<String>) {
    let text = text.trim();

    let Some(body) = text.strip_suffix('"') else {
        return (text, None);
    };
    let Some(open) = body.rfind('"') else {
        return (text, None);
    };

    let name = body[..open].trim();
    // Sin nada delante, las comillas eran el nombre del elemento.
    if name.is_empty() {
        return (text, None);
    }

    (name, Some(body[open + 1..].trim().to_string()))
}

/// `"n" base` -> (`base`, `n`).
pub(super) fn leading_multiplicity(text: &str) -> (&str, Option<String>) {
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

/// Un color de PlantUML llega sin la almohadilla. Los nombres (`LightBlue`)
/// valen tal cual en SVG; los hexadecimales hay que devolverselos.
fn normalize_color(raw: &str) -> String {
    let hex = matches!(raw.len(), 3 | 6 | 8) && raw.chars().all(|c| c.is_ascii_hexdigit());

    if hex {
        format!("#{raw}")
    } else {
        raw.to_string()
    }
}

/// Convierte un nombre en algo usable dentro de un id.
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

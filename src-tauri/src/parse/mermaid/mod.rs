//! Importador de `flowchart` / `graph` de Mermaid.
//!
//! Mermaid es un lenguaje de linea: cada una declara un nodo, una relacion o
//! una directiva. No hace falta un tokenizador general como el de DBML; lo
//! que hace falta es leer bien las formas, que se distinguen por los signos
//! que rodean la etiqueta, y las flechas, que se distinguen por su dibujo.
//!
//! Lo que no entiende no lo adivina: lo reporta con linea y columna y sigue
//! leyendo el resto.

pub(crate) mod shapes;

use std::collections::HashMap;

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, LayoutDirection, Node, NodeShape, Routing,
    Span,
};

use super::ImportResult;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Un `subgraph` abierto, con el id del paquete que lo representa.
struct OpenGroup {
    id: String,
    span: Span,
}

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Nombre tal cual aparece en el codigo -> indice en `document.nodes`.
    by_name: HashMap<String, usize>,
    /// Pila de `subgraph` abiertos.
    groups: Vec<OpenGroup>,
    /// Se vio ya la cabecera `flowchart` / `graph`.
    started: bool,
    next_edge: usize,
    next_group: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        Parser {
            document: Document::new(
                uuid::Uuid::new_v4().to_string(),
                DiagramKind::Architecture,
                now_ms,
            ),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            groups: Vec::new(),
            started: false,
            next_edge: 0,
            next_group: 0,
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
            let text = strip_comment(raw);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            // La columna del primer caracter con contenido, para que los
            // diagnosticos apunten a donde empieza lo que falla.
            let column = (text.len() - text.trim_start().len()) as u32 + 1;

            self.line(trimmed, line, column);
        }

        for group in std::mem::take(&mut self.groups) {
            self.diagnostics.push(
                Diagnostic::error(
                    "unterminatedSubgraph",
                    "Falta `end` para cerrar el `subgraph`.",
                )
                .at_span(group.span),
            );
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        let lower = text.to_lowercase();

        if !self.started {
            if let Some(rest) = lower
                .strip_prefix("flowchart")
                .or_else(|| lower.strip_prefix("graph"))
            {
                self.started = true;
                if let Some(direction) = parse_direction(rest.trim()) {
                    self.document.direction = direction;
                } else if !rest.trim().is_empty() {
                    self.warn(
                        "unknownDirection",
                        format!(
                            "Direccion `{}` desconocida; se usa la de por defecto.",
                            rest.trim()
                        ),
                        line,
                        column,
                    );
                }
                return;
            }

            // Sin cabecera se sigue leyendo igual: un fragmento pegado sin la
            // primera linea es un descuido corriente.
            self.started = true;
            self.warn(
                "missingHeader",
                "Falta la linea `flowchart` o `graph`; se asume un diagrama de flujo.",
                line,
                column,
            );
        }

        if lower == "end" {
            if self.groups.pop().is_none() {
                self.error(
                    "unexpectedEnd",
                    "`end` sin un `subgraph` que cerrar.",
                    line,
                    column,
                );
            }
            return;
        }

        if let Some(rest) = strip_keyword(text, "subgraph") {
            self.open_subgraph(rest.trim(), line, column);
            return;
        }

        // Directivas que no cambian la estructura del diagrama.
        for keyword in [
            "direction",
            "classdef",
            "class",
            "style",
            "linkstyle",
            "click",
        ] {
            if let Some(rest) = strip_keyword(&lower, keyword) {
                self.directive(keyword, rest.trim(), text, line, column);
                return;
            }
        }

        self.statement(text, line, column);
    }

    fn directive(&mut self, keyword: &str, rest: &str, original: &str, line: u32, column: u32) {
        match keyword {
            // `direction` dentro de un subgraph cambia solo ese bloque; a
            // nivel raiz, el diagrama entero.
            "direction" if self.groups.is_empty() => {
                if let Some(direction) = parse_direction(rest) {
                    self.document.direction = direction;
                }
            }
            "direction" => {}
            "style" => self.apply_style(original, line, column),
            other => self.warn(
                "ignoredDirective",
                format!("`{other}` se importa sin efecto: los estilos por clase no se conservan."),
                line,
                column,
            ),
        }
    }

    /// `style id fill:#f9f,stroke:#333`
    fn apply_style(&mut self, text: &str, line: u32, column: u32) {
        let mut parts = text.split_whitespace();
        parts.next();

        let Some(target) = parts.next() else {
            self.error(
                "badStyle",
                "Falta el nodo al que aplicar el estilo.",
                line,
                column,
            );
            return;
        };

        let Some(&index) = self.by_name.get(target) else {
            self.warn(
                "unknownStyleTarget",
                format!("`style` menciona `{target}`, que no existe."),
                line,
                column,
            );
            return;
        };

        for pair in parts.collect::<Vec<_>>().join(" ").split(',') {
            let Some((key, value)) = pair.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "fill" => self.document.nodes[index].style.fill = Some(value.to_string()),
                "stroke" => self.document.nodes[index].style.stroke = Some(value.to_string()),
                "color" => self.document.nodes[index].style.text_color = Some(value.to_string()),
                _ => {}
            }
        }
    }

    fn open_subgraph(&mut self, rest: &str, line: u32, column: u32) {
        // `subgraph id[Titulo]` o `subgraph Titulo`
        let (name, label) = match shapes::split_declaration(rest) {
            Some((id, shape)) => (id.to_string(), shape.label),
            None => (rest.to_string(), rest.to_string()),
        };

        if name.is_empty() {
            self.error(
                "missingSubgraphName",
                "Falta el nombre del `subgraph`.",
                line,
                column,
            );
            return;
        }

        self.next_group += 1;
        let id = format!("g{}", self.next_group);

        let mut node = Node::new(&id, label, NodeShape::Package);
        node.parent = self.groups.last().map(|g| g.id.clone());

        self.by_name.insert(name, self.document.nodes.len());
        self.document.nodes.push(node);

        self.groups.push(OpenGroup {
            id,
            span: Span::new(line, column),
        });
    }

    /// Una linea con nodos y, quiza, relaciones entre ellos.
    fn statement(&mut self, text: &str, line: u32, column: u32) {
        let pieces = match shapes::split_chain(text) {
            Ok(pieces) => pieces,
            Err(message) => {
                self.error("badStatement", message, line, column);
                return;
            }
        };

        if pieces.nodes.is_empty() {
            self.error(
                "badStatement",
                format!("No se entiende `{text}`."),
                line,
                column,
            );
            return;
        }

        let mut previous: Option<usize> = None;

        for (position, declaration) in pieces.nodes.iter().enumerate() {
            let index = self.node_for(declaration, line, column);

            if let Some(from) = previous {
                // Hay tantas flechas como huecos entre nodos.
                let link = &pieces.links[position - 1];
                self.add_edge(from, index, link);
            }

            previous = Some(index);
        }
    }

    /// Indice del nodo, creandolo si es la primera vez que aparece.
    fn node_for(&mut self, declaration: &shapes::Declaration, line: u32, column: u32) -> usize {
        if let Some(&index) = self.by_name.get(&declaration.name) {
            // Una segunda mencion con forma o etiqueta las actualiza: en
            // Mermaid es corriente declarar el nodo despues de usarlo.
            if let Some(shape) = &declaration.shape {
                let node = &mut self.document.nodes[index];
                node.shape = shape.shape;
                node.label = shape.label.clone();
            }
            return index;
        }

        if declaration.name.len() > 120 {
            self.warn(
                "longIdentifier",
                "El identificador es sospechosamente largo; revisa la linea.",
                line,
                column,
            );
        }

        let id = format!("n_{}", slug(&declaration.name));
        let (shape, label) = match &declaration.shape {
            Some(shape) => (shape.shape, shape.label.clone()),
            None => (NodeShape::Rectangle, declaration.name.clone()),
        };

        let mut node = Node::new(id, label, shape);
        node.parent = self.groups.last().map(|g| g.id.clone());

        let index = self.document.nodes.len();
        self.by_name.insert(declaration.name.clone(), index);
        self.document.nodes.push(node);

        index
    }

    fn add_edge(&mut self, from: usize, to: usize, link: &shapes::Link) {
        self.next_edge += 1;
        let id = format!("e{}", self.next_edge);

        let source = self.document.nodes[from].id.clone();
        let target = self.document.nodes[to].id.clone();

        let mut edge = Edge::new(id, source, target, link.kind);
        edge.routing = Routing::Orthogonal;
        if let Some(label) = &link.label {
            edge.label = Some(label.clone());
        }
        if link.thick {
            edge.style.stroke_width = Some(2.5);
        }
        if link.dashed {
            edge.style.dashed = Some(true);
        }

        self.document.edges.push(edge);
    }

    fn finish(mut self) -> ImportResult {
        // Un `subgraph` vacio no aporta nada y estorba al auto-layout.
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

/// Quita el comentario `%%` de una linea, respetando el que va entre comillas.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;

    for i in 0..bytes.len() {
        match bytes[i] {
            b'"' => quoted = !quoted,
            b'%' if !quoted && bytes.get(i + 1) == Some(&b'%') => return &line[..i],
            _ => {}
        }
    }

    line
}

/// Consume una palabra clave al principio, sin distinguir mayusculas.
fn strip_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let lower = text.to_lowercase();
    let rest = lower.strip_prefix(keyword)?;

    // `classDef` no debe disparar con `class`: tras la palabra tiene que
    // venir un espacio o el final de la linea.
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some(&text[keyword.len()..])
}

fn parse_direction(text: &str) -> Option<LayoutDirection> {
    match text.trim().to_uppercase().as_str() {
        "TD" | "TB" | "V" => Some(LayoutDirection::Down),
        "BT" => Some(LayoutDirection::Up),
        "LR" | ">" => Some(LayoutDirection::Right),
        "RL" | "<" => Some(LayoutDirection::Left),
        _ => None,
    }
}

/// Convierte un nombre en algo usable como id.
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

pub use shapes::delimiters_for;

/// Tipo de flecha segun su dibujo. Expuesto para el emisor.
pub fn arrow_for(kind: EdgeKind, dashed: bool, thick: bool) -> &'static str {
    let head = matches!(
        kind,
        EdgeKind::Directed | EdgeKind::Dependency | EdgeKind::Flow
    );

    match (dashed, thick, head) {
        (true, _, true) => "-.->",
        (true, _, false) => "-.-",
        (_, true, true) => "==>",
        (_, true, false) => "===",
        (_, _, true) => "-->",
        (_, _, false) => "---",
    }
}

#[cfg(test)]
mod tests;

//! Generador de maquinas de estados de PlantUML.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! La diferencia con el emisor de Mermaid esta en la etiqueta. En PlantUML,
//! `Nuevo : esperando pago` **anade** un renglon debajo del nombre en vez de
//! sustituirlo, asi que la etiqueta de un estado puede llevar varias lineas:
//! la primera es su nombre y el resto son descripciones. Al escribir se
//! vuelven a separar.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{Document, Edge, Node, NodeShape};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("@startuml\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "title {}", doc.meta.title.replace('\n', " "));
    }
    out.push('\n');

    let names = names_of(doc);
    emit_level(doc, None, 0, &names, &mut out);

    out.push_str("@enduml\n");
    out
}

/// Primera linea de la etiqueta: el nombre del estado.
fn caption_of(node: &Node) -> &str {
    node.label.lines().next().unwrap_or("")
}

/// Renglones de descripcion, si los hay.
fn descriptions_of(node: &Node) -> Vec<&str> {
    node.label
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .collect()
}

fn names_of(doc: &Document) -> HashMap<String, String> {
    let mut used: Vec<String> = Vec::new();
    let mut names = HashMap::new();

    for node in &doc.nodes {
        if is_terminal(node) {
            names.insert(node.id.clone(), "[*]".to_string());
            continue;
        }

        let mut name = sanitize(caption_of(node));
        if name.is_empty() {
            name = format!("s{}", used.len() + 1);
        }

        if used.contains(&name) {
            let base = name.clone();
            for extra in 2.. {
                name = format!("{base}_{extra}");
                if !used.contains(&name) {
                    break;
                }
            }
        }

        used.push(name.clone());
        names.insert(node.id.clone(), name);
    }

    names
}

fn is_terminal(node: &Node) -> bool {
    matches!(node.shape, NodeShape::InitialNode | NodeShape::FinalNode)
}

fn sanitize(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Una barra sale siempre como `<<fork>>`: en el documento una bifurcacion y
/// una union son la misma forma.
fn stereotype_of(shape: NodeShape) -> Option<&'static str> {
    match shape {
        NodeShape::Bar => Some("fork"),
        NodeShape::Diamond => Some("choice"),
        _ => None,
    }
}

fn emit_level(
    doc: &Document,
    parent: Option<&str>,
    depth: usize,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    let indent = INDENT.repeat(depth);

    for node in doc.children_of(parent) {
        if is_terminal(node) {
            continue;
        }

        let name = &names[&node.id];
        let caption = caption_of(node);

        if node.shape == NodeShape::Package {
            let head = if name == caption {
                format!("state {name}")
            } else {
                format!("state \"{}\" as {name}", escape(caption))
            };
            let _ = writeln!(out, "{indent}{head} {{");
            emit_level(doc, Some(&node.id), depth + 1, names, out);
            let _ = writeln!(out, "{indent}}}");
        } else if let Some(stereotype) = stereotype_of(node.shape) {
            let _ = writeln!(out, "{indent}state {name} <<{stereotype}>>");
        } else if name != caption {
            let _ = writeln!(out, "{indent}state \"{}\" as {name}", escape(caption));
        } else if !touched_by_any_edge(doc, &node.id) && descriptions_of(node).is_empty() {
            // Un estado al que ninguna transicion menciona y que tampoco
            // lleva descripcion necesita su linea; sin ella desapareceria.
            let _ = writeln!(out, "{indent}state {name}");
        }

        // Cada renglon de descripcion va en su propia linea, que es como
        // PlantUML los apila.
        for description in descriptions_of(node) {
            let _ = writeln!(out, "{indent}{name} : {description}");
        }
    }

    emit_edges(doc, parent, &indent, names, out);
}

fn touched_by_any_edge(doc: &Document, id: &str) -> bool {
    doc.edges
        .iter()
        .any(|e| e.source.node == id || e.target.node == id)
}

/// Compuesto al que pertenece una transicion: el de sus dos extremos cuando
/// coinciden, y la raiz cuando no.
///
/// Da igual para un estado corriente, pero no para un `[*]`: escrito en la
/// raiz seria el arranque del diagrama entero y no el del compuesto.
fn level_of<'a>(doc: &'a Document, edge: &Edge) -> Option<&'a str> {
    let from = doc.node(&edge.source.node)?.parent.as_deref();
    let to = doc.node(&edge.target.node)?.parent.as_deref();

    if from == to { from } else { None }
}

fn emit_edges(
    doc: &Document,
    parent: Option<&str>,
    indent: &str,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    for edge in &doc.edges {
        if level_of(doc, edge) != parent {
            continue;
        }

        let (Some(from), Some(to)) = (names.get(&edge.source.node), names.get(&edge.target.node))
        else {
            continue;
        };

        match &edge.label {
            Some(label) if !label.trim().is_empty() => {
                let _ = writeln!(
                    out,
                    "{indent}{from} --> {to} : {}",
                    label.trim().replace('\n', "\\n")
                );
            }
            _ => {
                let _ = writeln!(out, "{indent}{from} --> {to}");
            }
        }
    }
}

fn escape(label: &str) -> String {
    label.replace('"', "'")
}

#[cfg(test)]
mod tests;

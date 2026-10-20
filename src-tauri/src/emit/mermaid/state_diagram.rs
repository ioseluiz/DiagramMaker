//! Generador de `stateDiagram-v2` de Mermaid.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! Dos cosas que no son obvias:
//!
//! - Los extremos no se declaran: se escriben `[*]` alli donde aparecen, y
//!   cual de los dos sea lo dice el lado de la flecha.
//! - Una transicion se escribe **dentro** del compuesto al que pertenecen sus
//!   dos extremos. Da igual para un estado corriente, porque los nombres son
//!   globales, pero no para un `[*]`: escrito en la raiz seria el arranque
//!   del diagrama entero y no el del compuesto.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{Document, Edge, LayoutDirection, Node, NodeShape};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("stateDiagram-v2\n");
    let _ = writeln!(out, "{INDENT}direction {}", direction_code(doc.direction));

    let names = names_of(doc);
    emit_level(doc, None, 1, &names, &mut out);

    out
}

fn direction_code(direction: LayoutDirection) -> &'static str {
    match direction {
        LayoutDirection::Right => "LR",
        LayoutDirection::Left => "RL",
        LayoutDirection::Down => "TB",
        LayoutDirection::Up => "BT",
    }
}

/// Como se nombra cada nodo en el codigo.
///
/// Los extremos son siempre `[*]`. El resto usa su propia etiqueta, saneada
/// cuando Mermaid no la admitiria tal cual.
fn names_of(doc: &Document) -> HashMap<String, String> {
    let mut used: Vec<String> = Vec::new();
    let mut names = HashMap::new();

    for node in &doc.nodes {
        if is_terminal(node) {
            names.insert(node.id.clone(), "[*]".to_string());
            continue;
        }

        let mut name = sanitize(&node.label);
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

/// Estereotipo con el que se escribe una forma, si lo tiene.
///
/// Una barra sale siempre como `<<fork>>`: en el documento una bifurcacion y
/// una union son la misma forma, y elegir una al escribir no cambia lo que se
/// dibuja al volver a leerlo.
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
        // Los extremos no se declaran: aparecen en las transiciones.
        if is_terminal(node) {
            continue;
        }

        let name = &names[&node.id];

        if node.shape == NodeShape::Package {
            let _ = writeln!(out, "{indent}state {}{{", declaration(node, name));
            emit_level(doc, Some(&node.id), depth + 1, names, out);
            let _ = writeln!(out, "{indent}}}");
            continue;
        }

        if let Some(stereotype) = stereotype_of(node.shape) {
            let _ = writeln!(out, "{indent}state {name} <<{stereotype}>>");
            continue;
        }

        // Un estado corriente solo necesita su linea cuando el nombre no basta
        // —la etiqueta lleva espacios— o cuando ninguna transicion lo
        // menciona y sin ella desapareceria.
        if name != &node.label {
            let _ = writeln!(out, "{indent}state \"{}\" as {name}", escape(&node.label));
        } else if !touched_by_any_edge(doc, &node.id) {
            let _ = writeln!(out, "{indent}state {name}");
        }
    }

    emit_edges(doc, parent, &indent, names, out);
}

/// Cabecera de un compuesto: con descripcion si la etiqueta no vale como
/// nombre, y sin ella si vale.
fn declaration(node: &Node, name: &str) -> String {
    if name == node.label {
        format!("{name} ")
    } else {
        format!("\"{}\" as {name} ", escape(&node.label))
    }
}

fn touched_by_any_edge(doc: &Document, id: &str) -> bool {
    doc.edges
        .iter()
        .any(|e| e.source.node == id || e.target.node == id)
}

/// Compuesto al que pertenece una transicion: el de sus dos extremos cuando
/// coinciden, y la raiz cuando no.
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
                    label.trim().replace('\n', "<br/>")
                );
            }
            _ => {
                let _ = writeln!(out, "{indent}{from} --> {to}");
            }
        }
    }
}

fn escape(label: &str) -> String {
    label.replace('"', "'").replace('\n', " ")
}

#[cfg(test)]
mod tests;

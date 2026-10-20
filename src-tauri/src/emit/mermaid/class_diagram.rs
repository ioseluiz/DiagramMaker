//! Generador de `classDiagram` de Mermaid.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! A diferencia de los otros emisores, aqui los nombres **no** se acortan a
//! `n1`, `n2`: en un diagrama de clases el nombre de la clase es el nombre
//! del tipo, y leer `n3 <|-- n7` no dice nada. Se usa el propio nombre,
//! saneado solo donde Mermaid no admitiria lo que hay.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{ClassBox, ClassMember, Document, EdgeKind, LayoutDirection, Node, NodeShape};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("classDiagram\n");
    let _ = writeln!(out, "{INDENT}direction {}", direction_code(doc.direction));

    let names = names_of(doc);

    emit_level(doc, None, 1, &names, &mut out);
    emit_edges(doc, &names, &mut out);

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

/// Nombre con el que se referencia cada nodo en el codigo.
///
/// Mermaid admite casi cualquier cosa como nombre de clase, pero no espacios
/// ni signos: cuando la etiqueta los lleva, el nombre se sanea y la etiqueta
/// de verdad va entre corchetes.
fn names_of(doc: &Document) -> HashMap<String, String> {
    let mut used: Vec<String> = Vec::new();
    let mut names = HashMap::new();

    for node in &doc.nodes {
        let mut name = sanitize(&node.label);
        if name.is_empty() {
            name = format!("Clase{}", used.len() + 1);
        }

        // Dos etiquetas distintas pueden sanearse igual; el sufijo las separa.
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

/// Deja solo lo que Mermaid acepta en un nombre de clase.
fn sanitize(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Escribe las clases de un nivel y, recursivamente, las de sus namespaces.
fn emit_level(
    doc: &Document,
    parent: Option<&str>,
    depth: usize,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    let indent = INDENT.repeat(depth);

    for node in doc.children_of(parent) {
        if node.shape == NodeShape::Package {
            let _ = writeln!(out, "{indent}namespace {} {{", sanitize(&node.label));
            emit_level(doc, Some(&node.id), depth + 1, names, out);
            let _ = writeln!(out, "{indent}}}");
            continue;
        }

        emit_class(node, &indent, &names[&node.id], out);
    }
}

fn emit_class(node: &Node, indent: &str, name: &str, out: &mut String) {
    let empty = ClassBox::new();
    let box_ = node.class().unwrap_or(&empty);

    // Si el nombre tuvo que sanearse, la etiqueta de verdad va entre
    // corchetes; si no, sobra y solo seria ruido.
    let label = if name == node.label {
        String::new()
    } else {
        format!("[\"{}\"]", node.label.replace('"', "'"))
    };

    let members = box_.attributes.len() + box_.operations.len();
    if members == 0 && !box_.has_stereotype() {
        let _ = writeln!(out, "{indent}class {name}{label}");
        return;
    }

    let _ = writeln!(out, "{indent}class {name}{label} {{");

    if let Some(stereotype) = box_.stereotype.as_deref().filter(|s| !s.trim().is_empty()) {
        let _ = writeln!(out, "{indent}{INDENT}<<{}>>", stereotype.trim());
    }
    for member in &box_.attributes {
        let _ = writeln!(out, "{indent}{INDENT}{}", member_text(member, false));
    }
    for member in &box_.operations {
        let _ = writeln!(out, "{indent}{INDENT}{}", member_text(member, true));
    }

    let _ = writeln!(out, "{indent}}}");
}

/// Como escribe Mermaid un miembro: el tipo delante del nombre en un
/// atributo, y detras de los parentesis en una operacion.
fn member_text(member: &ClassMember, is_operation: bool) -> String {
    let mut out = String::with_capacity(32);
    out.push(member.visibility.sigil());

    if is_operation {
        let _ = write!(out, "{}({})", member.name, member.parameters);
        if !member.type_name.is_empty() {
            let _ = write!(out, " {}", member.type_name);
        }
    } else {
        if !member.type_name.is_empty() {
            let _ = write!(out, "{} ", member.type_name);
        }
        out.push_str(&member.name);
    }

    // El clasificador va al final de todo, detras del nombre.
    if member.is_static {
        out.push('$');
    }
    if member.is_abstract {
        out.push('*');
    }

    out
}

fn emit_edges(doc: &Document, names: &HashMap<String, String>, out: &mut String) {
    if doc.edges.is_empty() {
        return;
    }
    out.push('\n');

    for edge in &doc.edges {
        let (Some(from), Some(to)) = (names.get(&edge.source.node), names.get(&edge.target.node))
        else {
            continue;
        };

        let dashed = edge
            .style
            .dashed
            .unwrap_or_else(|| edge.kind.is_dashed_by_default());

        let _ = write!(out, "{INDENT}{from}{}", multiplicity(&edge.source.label));
        let _ = write!(out, " {}", arrow_for(edge.kind, dashed));
        let _ = write!(out, "{} {to}", multiplicity(&edge.target.label));

        match &edge.label {
            Some(label) if !label.trim().is_empty() => {
                let _ = writeln!(out, " : {}", label.trim().replace('\n', "<br/>"));
            }
            _ => out.push('\n'),
        }
    }
}

/// Como se dibuja cada relacion.
///
/// Se escriben siempre con la punta a la derecha —`--|>`, no `<|--`— para que
/// el origen del documento sea el de la izquierda y no haya que invertir
/// nada al volver a leerlo.
fn arrow_for(kind: EdgeKind, dashed: bool) -> &'static str {
    match kind {
        EdgeKind::Inheritance => "--|>",
        EdgeKind::Realization => "..|>",
        EdgeKind::Composition => "*--",
        EdgeKind::Aggregation => "o--",
        EdgeKind::Dependency => "..>",
        EdgeKind::Directed | EdgeKind::Flow => "-->",
        EdgeKind::Association if dashed => "..",
        EdgeKind::Association => "--",
    }
}

fn multiplicity(label: &Option<String>) -> String {
    match label {
        Some(text) if !text.trim().is_empty() => format!(" \"{}\"", text.trim()),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;

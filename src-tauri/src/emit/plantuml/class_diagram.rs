//! Generador de diagramas de clases de PlantUML.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! El estereotipo decide con que palabra se declara la clase: `interface`,
//! `enum` o `abstract class` en vez de `class` con un `<<...>>` detras. Es la
//! vuelta de lo que hace el importador, y lo que hace que el codigo generado
//! se parezca al que escribiria una persona.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{ClassBox, ClassMember, Document, Node, NodeShape};
use crate::parse::plantuml::arrow_for;

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
    emit_edges(doc, &names, &mut out);

    out.push_str("@enduml\n");
    out
}

/// Nombre con el que se referencia cada nodo.
///
/// Igual que en el emisor de Mermaid, no se acortan: en un diagrama de
/// clases el nombre de la clase es el nombre del tipo. Cuando la etiqueta
/// lleva espacios, el nombre se sanea y la etiqueta de verdad va
/// entrecomillada con un `as`.
fn names_of(doc: &Document) -> HashMap<String, String> {
    let mut used: Vec<String> = Vec::new();
    let mut names = HashMap::new();

    for node in &doc.nodes {
        let mut name = sanitize(&node.label);
        if name.is_empty() {
            name = format!("Clase{}", used.len() + 1);
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

fn sanitize(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Con que palabra se declara una clase segun su estereotipo.
///
/// Devuelve tambien si el estereotipo hay que escribirlo ademas entre dobles
/// angulos: los que tienen palabra propia, no.
fn declaration(stereotype: Option<&str>) -> (&'static str, bool) {
    match stereotype.map(str::trim) {
        Some("interface") => ("interface", false),
        Some("abstract") => ("abstract class", false),
        Some("enumeration") | Some("enum") => ("enum", false),
        Some("annotation") => ("annotation", false),
        Some("struct") => ("struct", false),
        Some("protocol") => ("protocol", false),
        Some("exception") => ("exception", false),
        Some("metaclass") => ("metaclass", false),
        Some(other) if !other.is_empty() => ("class", true),
        _ => ("class", false),
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
        if node.shape == NodeShape::Package {
            let _ = writeln!(out, "{indent}package \"{}\" {{", escape(&node.label));
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

    let (keyword, write_stereotype) = declaration(box_.stereotype.as_deref());

    // Si el nombre tuvo que sanearse, la etiqueta de verdad va entrecomillada
    // y el nombre queda como alias.
    let head = if name == node.label {
        format!("{keyword} {name}")
    } else {
        format!("{keyword} \"{}\" as {name}", escape(&node.label))
    };

    let stereotype = if write_stereotype {
        format!(" <<{}>>", box_.stereotype.as_deref().unwrap_or("").trim())
    } else {
        String::new()
    };

    if box_.attributes.is_empty() && box_.operations.is_empty() {
        let _ = writeln!(out, "{indent}{head}{stereotype}");
        return;
    }

    let _ = writeln!(out, "{indent}{head}{stereotype} {{");

    for member in &box_.attributes {
        let _ = writeln!(out, "{indent}{INDENT}{}", member_text(member, false));
    }
    // El separador deja ver donde acaban los atributos; PlantUML lo ignora
    // si no hay nada detras, asi que solo va cuando hay las dos cosas.
    if !box_.attributes.is_empty() && !box_.operations.is_empty() {
        let _ = writeln!(out, "{indent}{INDENT}--");
    }
    for member in &box_.operations {
        let _ = writeln!(out, "{indent}{INDENT}{}", member_text(member, true));
    }

    let _ = writeln!(out, "{indent}}}");
}

/// Como escribe PlantUML un miembro: el tipo detras de los dos puntos y los
/// modificadores entre llaves.
fn member_text(member: &ClassMember, is_operation: bool) -> String {
    let mut out = String::with_capacity(40);
    out.push(member.visibility.sigil());
    out.push(' ');

    if member.is_static {
        out.push_str("{static} ");
    }
    if member.is_abstract {
        out.push_str("{abstract} ");
    }

    out.push_str(&member.name);
    if is_operation {
        let _ = write!(out, "({})", member.parameters);
    }
    if !member.type_name.is_empty() {
        let _ = write!(out, " : {}", member.type_name);
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

        let _ = write!(out, "{from}{}", multiplicity(&edge.source.label));
        let _ = write!(out, " {}", arrow_for(edge.kind, dashed));
        let _ = write!(out, "{} {to}", multiplicity(&edge.target.label));

        match &edge.label {
            Some(label) if !label.trim().is_empty() => {
                let _ = writeln!(out, " : {}", label.trim().replace('\n', "\\n"));
            }
            _ => out.push('\n'),
        }
    }
}

fn multiplicity(label: &Option<String>) -> String {
    match label {
        Some(text) if !text.trim().is_empty() => format!(" \"{}\"", escape(text.trim())),
        _ => String::new(),
    }
}

fn escape(text: &str) -> String {
    text.replace('"', "'").replace('\n', " ")
}

#[cfg(test)]
mod tests;

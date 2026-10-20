//! Generador de `erDiagram` de Mermaid.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! Dos cosas no vuelven iguales, y las dos por el idioma de destino:
//!
//! - **La columna concreta de una relacion.** El documento puede saber que la
//!   relacion va de `pedidos.cliente_id` a `clientes.id` —DBML lo dice— pero
//!   en Mermaid una relacion une entidades y no columnas. El puerto se
//!   pierde; lo que si se escribe es el `FK` del atributo, que es como
//!   Mermaid cuenta lo mismo.
//! - **Una columna sin tipo.** Mermaid exige `tipo nombre`: una sola palabra
//!   no es un atributo valido y rompe el dibujo entero. Antes que inventar un
//!   tipo plausible o perder la columna, se escribe `sin_tipo`, que dice la
//!   verdad y se ve. No se deshace al volver a leer, asi que esa columna
//!   regresa con ese tipo: es una normalizacion a la vista, no un apaño
//!   escondido.

use std::fmt::Write as _;

use crate::model::{Document, ErColumn, LayoutDirection, Node};

/// Sangria del cuerpo.
const INDENT: &str = "  ";

/// Lo que se escribe donde el usuario no ha puesto tipo.
const NO_TYPE: &str = "sin_tipo";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("erDiagram\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "{INDENT}title {}", doc.meta.title.replace('\n', " "));
    }
    if doc.direction != LayoutDirection::default() {
        let _ = writeln!(out, "{INDENT}direction {}", direction_code(doc.direction));
    }

    emit_relations(doc, &mut out);
    emit_entities(doc, &mut out);

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

/// Como se nombra una entidad en el codigo: su identificador.
///
/// Si la tabla guarda un alias, ese es el identificador y la etiqueta es el
/// texto que se dibuja. Sin alias, el identificador es la propia etiqueta,
/// entrecomillada si lleva algo que Mermaid no admite suelto.
fn reference(node: &Node) -> String {
    match node.er().and_then(|t| t.alias.as_deref()) {
        Some(alias) if is_plain(alias) => alias.to_string(),
        Some(alias) => quoted(alias),
        None if is_plain(&node.label) => node.label.clone(),
        None => quoted(&node.label),
    }
}

/// Declaracion de la entidad, con su alias si lo tiene.
fn declaration(node: &Node) -> String {
    match node.er().and_then(|t| t.alias.as_deref()) {
        Some(_) => format!("{}[{}]", reference(node), quoted(&node.label)),
        None => reference(node),
    }
}

/// `true` si el nombre puede escribirse sin comillas.
fn is_plain(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", escape(text))
}

/// Las comillas se cuentan al leer, asi que las sueltas se cambian.
fn escape(text: &str) -> String {
    text.replace('"', "'").replace('\n', " ")
}

fn emit_relations(doc: &Document, out: &mut String) {
    let mut wrote_any = false;

    for edge in &doc.edges {
        let (Some(source), Some(target)) =
            (doc.node(&edge.source.node), doc.node(&edge.target.node))
        else {
            continue;
        };

        let line = if edge.style.dashed == Some(true) {
            ".."
        } else {
            "--"
        };

        // La etiqueta se escribe siempre, vacia incluida: Mermaid la pide en
        // toda relacion y no cuesta nada dejarla puesta.
        let label = edge.label.as_deref().map(str::trim).unwrap_or("");

        let _ = writeln!(
            out,
            "{INDENT}{} {}{line}{} {} : {}",
            reference(source),
            left_symbol(edge.source.label.as_deref()),
            right_symbol(edge.target.label.as_deref()),
            reference(target),
            quoted(label),
        );
        wrote_any = true;
    }

    if wrote_any {
        out.push('\n');
    }
}

/// `||` o `}o`. El documento solo distingue uno de varios, asi que de los
/// cuatro simbolos de Mermaid se escriben los dos mas corrientes.
fn left_symbol(label: Option<&str>) -> &'static str {
    match label {
        Some("*") => "}o",
        _ => "||",
    }
}

fn right_symbol(label: Option<&str>) -> &'static str {
    match label {
        Some("*") => "o{",
        _ => "||",
    }
}

fn emit_entities(doc: &Document, out: &mut String) {
    for node in &doc.nodes {
        let Some(table) = node.er() else {
            continue;
        };

        if table.columns.is_empty() {
            // Sin atributos no hace falta bloque; la entidad ya existe por su
            // relacion. Solo se declara si no la nombra ninguna.
            if !touched_by_any_edge(doc, &node.id) {
                let _ = writeln!(out, "{INDENT}{}", declaration(node));
            }
            continue;
        }

        let _ = writeln!(out, "{INDENT}{} {{", declaration(node));
        for column in &table.columns {
            let _ = writeln!(out, "{INDENT}{INDENT}{}", attribute(column));
        }
        let _ = writeln!(out, "{INDENT}}}");
    }
}

fn touched_by_any_edge(doc: &Document, id: &str) -> bool {
    doc.edges
        .iter()
        .any(|e| e.source.node == id || e.target.node == id)
}

fn attribute(column: &ErColumn) -> String {
    let data_type = if column.data_type.trim().is_empty() {
        NO_TYPE.to_string()
    } else {
        word(&column.data_type)
    };

    let mut out = format!("{data_type} {}", word(&column.name));

    let mut keys = Vec::new();
    if column.primary_key {
        keys.push("PK");
    }
    if column.foreign_key {
        keys.push("FK");
    }
    if column.unique && !column.primary_key {
        keys.push("UK");
    }
    if !keys.is_empty() {
        let _ = write!(out, " {}", keys.join(", "));
    }

    if let Some(note) = column
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        let _ = write!(out, " {}", quoted(note));
    }

    out
}

/// Un tipo o un nombre de atributo van sueltos, sin comillas: lo que Mermaid
/// no admite ahi se junta con guion bajo en vez de partir la linea en dos.
fn word(text: &str) -> String {
    let cleaned: String = text
        .trim()
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .filter(|c| *c != '"')
        .collect();

    if cleaned.is_empty() {
        NO_TYPE.to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests;

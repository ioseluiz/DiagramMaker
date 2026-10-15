//! Generador de `sequenceDiagram` de Mermaid.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! Lo unico que no se escribe como entro es la flecha: Mermaid tiene varias
//! para lo mismo —`->` y `->>` son las dos una llamada— y el documento solo
//! guarda el significado, asi que sale siempre la cannica de cada tipo.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::model::{
    Block, BlockKind, Document, Message, MessageKind, NodeShape, NotePlacement, SequenceNote, Step,
};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("sequenceDiagram\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "{INDENT}title {}", one_line(&doc.meta.title));
    }

    // Nombres cortos y estables: `n1`, `n2`... Los ids del documento llevan
    // prefijos internos que en el codigo solo serian ruido.
    let names = short_names(doc);
    let script = doc.script.clone().unwrap_or_default();

    // Un participante que nace a mitad del guion no se declara arriba: su
    // linea `create` va donde le llega el mensaje.
    let born: HashSet<&str> = script
        .messages()
        .iter()
        .filter(|m| m.kind == MessageKind::Create)
        .map(|m| m.to.as_str())
        .collect();

    for node in &doc.nodes {
        if born.contains(node.id.as_str()) {
            continue;
        }
        let _ = writeln!(
            out,
            "{INDENT}{} {} as {}",
            declaration(node.shape),
            names[&node.id],
            one_line(&node.label)
        );
    }

    if !script.steps.is_empty() {
        out.push('\n');
    }
    emit_steps(&script.steps, 1, doc, &names, &mut out);

    out
}

/// Con que palabra se declara cada forma.
fn declaration(shape: NodeShape) -> &'static str {
    match shape {
        NodeShape::Actor => "actor",
        _ => "participant",
    }
}

fn short_names(doc: &Document) -> HashMap<String, String> {
    doc.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("n{}", index + 1)))
        .collect()
}

fn emit_steps(
    steps: &[Step],
    depth: usize,
    doc: &Document,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    for step in steps {
        match step {
            Step::Message(message) => emit_message(message, depth, doc, names, out),
            Step::Note(note) => emit_note(note, depth, names, out),
            Step::Block(block) => emit_block(block, depth, doc, names, out),
        }
    }
}

fn emit_message(
    message: &Message,
    depth: usize,
    doc: &Document,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    let indent = INDENT.repeat(depth);
    let (Some(from), Some(to)) = (names.get(&message.from), names.get(&message.to)) else {
        return;
    };

    // `create` y `destroy` son lineas propias en Mermaid, no tipos de flecha.
    match message.kind {
        MessageKind::Create => {
            let label = doc
                .node(&message.to)
                .map(|n| one_line(&n.label))
                .unwrap_or_default();
            let word = doc
                .node(&message.to)
                .map_or("participant", |n| declaration(n.shape));
            let _ = writeln!(out, "{indent}create {word} {to} as {label}");
        }
        MessageKind::Destroy => {
            let _ = writeln!(out, "{indent}destroy {to}");
        }
        _ => {}
    }

    // El signo pegado al destino abre o cierra ejecucion. Solo cabe uno: si
    // el mensaje hace las dos cosas, la segunda va en su propia linea.
    let (mark, extra) = match (message.activate, message.deactivate) {
        (true, false) => ("+", None),
        (false, true) => ("-", None),
        (true, true) => ("+", Some(format!("{indent}deactivate {from}\n"))),
        (false, false) => ("", None),
    };

    let text = one_line(&message.text);
    let _ = writeln!(
        out,
        "{indent}{from}{}{mark}{to}{}",
        arrow_for(message.kind),
        if text.is_empty() {
            String::new()
        } else {
            format!(": {text}")
        }
    );

    if let Some(line) = extra {
        out.push_str(&line);
    }
}

/// Como se dibuja cada tipo de mensaje.
///
/// `create` y `destroy` no tienen flecha propia: su linea va aparte y el
/// mensaje en si es una llamada corriente.
fn arrow_for(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Reply => "-->>",
        MessageKind::Async => "-)",
        _ => "->>",
    }
}

fn emit_note(note: &SequenceNote, depth: usize, names: &HashMap<String, String>, out: &mut String) {
    let anchors: Vec<&str> = note
        .participants
        .iter()
        .filter_map(|id| names.get(id).map(String::as_str))
        .collect();
    if anchors.is_empty() {
        return;
    }

    let placement = match note.placement {
        NotePlacement::LeftOf => "left of",
        NotePlacement::RightOf => "right of",
        NotePlacement::Over => "over",
    };

    let _ = writeln!(
        out,
        "{}Note {placement} {}: {}",
        INDENT.repeat(depth),
        anchors.join(","),
        one_line(&note.text)
    );
}

fn emit_block(
    block: &Block,
    depth: usize,
    doc: &Document,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    let indent = INDENT.repeat(depth);

    for (position, branch) in block.branches.iter().enumerate() {
        let word = if position == 0 {
            opening(block.kind)
        } else {
            divider(block.kind)
        };

        let label = one_line(&branch.label);
        let _ = writeln!(
            out,
            "{indent}{word}{}",
            if label.is_empty() {
                String::new()
            } else {
                format!(" {label}")
            }
        );

        emit_steps(&branch.steps, depth + 1, doc, names, out);
    }

    let _ = writeln!(out, "{indent}end");
}

fn opening(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Alt => "alt",
        BlockKind::Opt => "opt",
        BlockKind::Loop => "loop",
        BlockKind::Par => "par",
        BlockKind::Critical => "critical",
        BlockKind::Break => "break",
        // Mermaid no tiene `group` en secuencia; `rect` es lo que agrupa, y
        // necesita un color.
        BlockKind::Group => "rect rgb(248, 248, 250)",
    }
}

fn divider(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Par => "and",
        BlockKind::Critical => "option",
        _ => "else",
    }
}

/// Mermaid escribe los saltos de linea con la etiqueta de HTML.
fn one_line(text: &str) -> String {
    text.trim().replace('\n', "<br/>")
}

#[cfg(test)]
mod tests;

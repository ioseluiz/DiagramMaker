//! Generador de diagramas de secuencia de PlantUML.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! Lo propio de PlantUML es que la apertura y el cierre de una ejecucion van
//! pegados al destinatario escrito: `A -> B ++` abre la de B y `B --> A --`
//! cierra la de B. Como caben los dos a la vez (`--++`), un mensaje que
//! abre y cierra cabe en una sola linea, cosa que en Mermaid no pasa.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::model::{
    Block, BlockKind, Document, Message, MessageKind, NodeShape, NotePlacement, SequenceNote, Step,
};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("@startuml\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "title {}", one_line(&doc.meta.title));
    }

    // Alias cortos y estables: `n1`, `n2`... Los ids del documento llevan
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
            "{} \"{}\" as {}",
            keyword(node.shape),
            escape(&node.label),
            names[&node.id]
        );
    }

    if !script.steps.is_empty() {
        out.push('\n');
    }
    emit_steps(&script.steps, 0, doc, &names, &mut out);

    out.push_str("@enduml\n");
    out
}

/// Con que palabra se declara cada forma.
fn keyword(shape: NodeShape) -> &'static str {
    match shape {
        NodeShape::Actor => "actor",
        NodeShape::Cylinder => "database",
        NodeShape::Hexagon => "queue",
        NodeShape::Ellipse => "control",
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

    // `create` es una linea propia; la destruccion cabe en un sufijo.
    if message.kind == MessageKind::Create {
        let node = doc.node(&message.to);
        let word = node.map_or("participant", |n| keyword(n.shape));
        let label = node.map(|n| escape(&n.label)).unwrap_or_default();
        let _ = writeln!(out, "{indent}create {word} \"{label}\" as {to}");
    }

    // Los sufijos van pegados al destinatario y caben juntos: `--` cierra la
    // del origen y `++` abre la del destino.
    let mut marks = String::new();
    if message.deactivate {
        marks.push_str(" --");
    }
    if message.activate {
        marks.push_str(if marks.is_empty() { " ++" } else { "++" });
    }
    if message.kind == MessageKind::Destroy {
        marks.push_str(if marks.is_empty() { " !!" } else { "!!" });
    }

    let text = one_line(&message.text);
    let _ = writeln!(
        out,
        "{indent}{from} {} {to}{marks}{}",
        arrow_for(message.kind),
        if text.is_empty() {
            String::new()
        } else {
            format!(" : {text}")
        }
    );
}

/// Como se dibuja cada tipo de mensaje.
///
/// `create` y `destroy` no tienen flecha propia: lo que los marca es la
/// linea `create` o el sufijo `!!`, y la flecha en si es una llamada.
fn arrow_for(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Reply => "-->",
        MessageKind::Async => "->>",
        _ => "->",
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
        "{}note {placement} {} : {}",
        INDENT.repeat(depth),
        anchors.join(", "),
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
        // PlantUML divide con `else` cualquier marco, tambien un `par`.
        let word = if position == 0 {
            opening(block.kind)
        } else {
            "else"
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
        BlockKind::Group => "group",
    }
}

/// PlantUML escribe los saltos de linea con `\n` dentro del texto.
fn one_line(text: &str) -> String {
    text.trim().replace('\n', "\\n")
}

/// Un nombre entrecomillado no admite comillas dentro.
fn escape(text: &str) -> String {
    text.replace('"', "'").replace('\n', " ")
}

#[cfg(test)]
mod tests;

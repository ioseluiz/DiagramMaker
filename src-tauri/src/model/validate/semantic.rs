//! Comprobaciones propias de cada tipo de diagrama.
//!
//! Lo estructural —identificadores unicos, referencias que existen, ciclos de
//! anidamiento— vale para cualquier documento y vive en el modulo de arriba.
//! Aqui esta lo que solo tiene sentido sabiendo que se esta dibujando: que un
//! rombo que no decide nada es un rombo de mas, que una maquina de estados
//! con dos arranques no se puede leer, que un caso de uso al que no llega
//! ningun actor probablemente falta conectarlo.
//!
//! Tres reglas sobre que es error y que es aviso, para que la lista sirva de
//! algo y no se convierta en ruido que se aprende a ignorar:
//!
//! - **Error** solo si el diagrama se contradice: dos arranques, una herencia
//!   circular, un mensaje a un participante que no existe. Son cosas que no
//!   significan nada, no cuestiones de gusto.
//! - **Aviso** para lo que se dibuja bien pero se notaria al exportar o al
//!   leerlo: una tabla vacia, un caso de uso suelto, un paso sin salida.
//! - **Nada** para lo que es decision del autor. Un diagrama a medias es un
//!   diagrama a medias, no un error; se esta dibujando.

use std::collections::{HashMap, HashSet};

use crate::model::diagnostic::Diagnostic;
use crate::model::document::{DiagramKind, Document};
use crate::model::edge::EdgeKind;
use crate::model::node::NodeShape;
use crate::model::sequence::Step;

pub fn check(doc: &Document, out: &mut Vec<Diagnostic>) {
    match doc.kind {
        DiagramKind::Er => er(doc, out),
        DiagramKind::Class => class(doc, out),
        DiagramKind::State => {
            terminals(doc, out);
            state(doc, out);
        }
        DiagramKind::Activity => {
            terminals(doc, out);
            activity(doc, out);
        }
        DiagramKind::UseCase => use_case(doc, out),
        DiagramKind::Sequence => sequence(doc, out),
        DiagramKind::Architecture => {}
    }
}

/* --- Entidad-relacion -------------------------------------------------------- */

fn er(doc: &Document, out: &mut Vec<Diagnostic>) {
    for node in &doc.nodes {
        let Some(table) = node.er() else { continue };

        if table.columns.is_empty() {
            out.push(
                Diagnostic::warning(
                    "emptyTable",
                    format!("La tabla `{}` no tiene columnas.", node.label),
                )
                .at(&node.id),
            );
        }

        // Dos columnas con el mismo nombre no son dos columnas: ni DBML ni
        // ninguna base de datos sabria cual es cual.
        let mut seen: HashSet<String> = HashSet::new();
        for column in &table.columns {
            let key = column.name.to_lowercase();
            if !seen.insert(key) {
                out.push(
                    Diagnostic::error(
                        "duplicateColumn",
                        format!(
                            "La tabla `{}` tiene dos columnas llamadas `{}`.",
                            node.label, column.name
                        ),
                    )
                    .at(&node.id),
                );
            }
        }
    }

    for edge in &doc.edges {
        for endpoint in [&edge.source, &edge.target] {
            let Some(node) = doc.node(&endpoint.node) else {
                continue;
            };
            if node.er().is_none() && node.shape != NodeShape::Package {
                out.push(
                    Diagnostic::warning(
                        "relationOutsideTable",
                        format!(
                            "La relacion toca `{}`, que no es una tabla; no saldra en el DBML.",
                            node.label
                        ),
                    )
                    .at(&edge.id),
                );
            }
        }
    }
}

/* --- Clases -------------------------------------------------------------------- */

fn class(doc: &Document, out: &mut Vec<Diagnostic>) {
    // Una clase que hereda de si misma, directa o dando la vuelta, no tiene
    // lectura posible.
    let parents: HashMap<&str, Vec<&str>> = doc
        .edges
        .iter()
        .filter(|e| matches!(e.kind, EdgeKind::Inheritance | EdgeKind::Realization))
        .fold(HashMap::new(), |mut acc, e| {
            acc.entry(e.source.node.as_str())
                .or_default()
                .push(e.target.node.as_str());
            acc
        });

    let mut reported: HashSet<&str> = HashSet::new();

    for &start in parents.keys() {
        if reported.contains(start) {
            continue;
        }
        let Some(cycle) = cycle_from(start, &parents) else {
            continue;
        };

        let names: Vec<String> = cycle
            .iter()
            .map(|id| {
                doc.node(id)
                    .map(|n| n.label.clone())
                    .unwrap_or_else(|| (*id).to_string())
            })
            .collect();

        for id in &cycle {
            reported.insert(id);
        }

        out.push(
            Diagnostic::error(
                "inheritanceCycle",
                format!("Herencia circular: {}.", names.join(" → ")),
            )
            .at(cycle[0]),
        );
    }
}

/// Primer ciclo que se encuentra saliendo de un nodo, si lo hay.
fn cycle_from<'a>(start: &'a str, edges: &HashMap<&'a str, Vec<&'a str>>) -> Option<Vec<&'a str>> {
    let mut path = vec![start];
    let mut cursor = start;

    // Se sigue un solo camino: con herencia multiple puede quedar algun ciclo
    // sin ver, pero el primero basta para que el autor mire ahi.
    while let Some(next) = edges.get(cursor).and_then(|targets| targets.first()) {
        if let Some(at) = path.iter().position(|id| id == next) {
            let mut cycle = path[at..].to_vec();
            cycle.push(next);
            return Some(cycle);
        }
        path.push(next);
        cursor = next;
    }

    None
}

/* --- Extremos de un flujo ------------------------------------------------------ */

/// Reglas que comparten las maquinas de estados y los diagramas de actividad:
/// por el arranque solo se sale y por el final solo se entra.
fn terminals(doc: &Document, out: &mut Vec<Diagnostic>) {
    let entra = |id: &str| doc.edges.iter().any(|e| e.target.node == id);
    let sale = |id: &str| doc.edges.iter().any(|e| e.source.node == id);

    for node in &doc.nodes {
        match node.shape {
            NodeShape::InitialNode if entra(&node.id) => out.push(
                Diagnostic::error(
                    "arrowIntoStart",
                    "Al arranque no entra ninguna flecha: es por donde se empieza.",
                )
                .at(&node.id),
            ),
            NodeShape::FinalNode if sale(&node.id) => out.push(
                Diagnostic::error(
                    "arrowOutOfEnd",
                    "Del final no sale ninguna flecha: ahi se acaba.",
                )
                .at(&node.id),
            ),
            _ => {}
        }
    }
}

/* --- Maquinas de estados --------------------------------------------------------- */

fn state(doc: &Document, out: &mut Vec<Diagnostic>) {
    // Hay un arranque por nivel de anidamiento, no uno por diagrama: un
    // estado compuesto tiene el suyo.
    let mut starts: HashMap<Option<&str>, Vec<&str>> = HashMap::new();
    for node in &doc.nodes {
        if node.shape == NodeShape::InitialNode {
            starts
                .entry(node.parent.as_deref())
                .or_default()
                .push(node.id.as_str());
        }
    }

    for (parent, ids) in &starts {
        if ids.len() < 2 {
            continue;
        }

        let donde = parent
            .and_then(|id| doc.node(id))
            .map(|n| format!("`{}`", n.label))
            .unwrap_or_else(|| "el diagrama".into());

        out.push(
            Diagnostic::error(
                "manyStarts",
                format!(
                    "{donde} tiene {} arranques; solo puede haber uno.",
                    ids.len()
                ),
            )
            .at(ids[1]),
        );
    }

    unreachable(doc, out, "unreachableState", "estado");
}

/* --- Actividad --------------------------------------------------------------------- */

fn activity(doc: &Document, out: &mut Vec<Diagnostic>) {
    if !doc.nodes.is_empty() && !doc.nodes.iter().any(|n| n.shape == NodeShape::InitialNode) {
        out.push(Diagnostic::warning(
            "noStart",
            "El diagrama no tiene arranque: no se sabe por donde empieza el flujo.",
        ));
    }

    for node in &doc.nodes {
        let salen = doc
            .edges
            .iter()
            .filter(|e| e.source.node == node.id)
            .count();

        match node.shape {
            // Un rombo con una sola salida no decide nada, y con ninguna deja
            // el flujo en el aire.
            NodeShape::Diamond if salen < 2 => out.push(
                Diagnostic::warning(
                    "decisionWithoutBranches",
                    format!(
                        "La condicion `{}` tiene {salen} salida(s); una decision necesita dos.",
                        node.label
                    ),
                )
                .at(&node.id),
            ),
            NodeShape::Bar if salen == 0 => out.push(
                Diagnostic::warning(
                    "forkWithoutBranches",
                    "La barra no reparte en ninguna rama.",
                )
                .at(&node.id),
            ),
            NodeShape::FinalNode | NodeShape::Package | NodeShape::Note | NodeShape::Text => {}
            _ if salen == 0 => out.push(
                Diagnostic::warning(
                    "danglingStep",
                    format!(
                        "De `{}` no sale nada y no es un final: el flujo se queda colgando.",
                        node.label
                    ),
                )
                .at(&node.id),
            ),
            _ => {}
        }
    }

    unreachable(doc, out, "unreachableStep", "paso");
}

/* --- Casos de uso ------------------------------------------------------------------- */

fn use_case(doc: &Document, out: &mut Vec<Diagnostic>) {
    let shape_of = |id: &str| doc.node(id).map(|n| n.shape);

    for edge in &doc.edges {
        let from = shape_of(&edge.source.node);
        let to = shape_of(&edge.target.node);

        let entre_casos = from == Some(NodeShape::Ellipse) && to == Some(NodeShape::Ellipse);
        let con_actor = from == Some(NodeShape::Actor) || to == Some(NodeShape::Actor);

        // Entre dos casos de uso la relacion es «include» o «extend», que se
        // dibujan con una dependencia; una asociacion ahi no dice nada.
        if entre_casos && matches!(edge.kind, EdgeKind::Association | EdgeKind::Directed) {
            out.push(
                Diagnostic::warning(
                    "associationBetweenUseCases",
                    "Entre dos casos de uso la relacion es «include» o «extend»: una dependencia.",
                )
                .at(&edge.id),
            );
        }

        if con_actor && edge.kind == EdgeKind::Dependency {
            out.push(
                Diagnostic::warning(
                    "dependencyWithActor",
                    "Lo que une a un actor con un caso de uso es una asociacion, no una dependencia.",
                )
                .at(&edge.id),
            );
        }
    }

    // Un caso de uso al que no llega ningun actor no lo usa nadie. Los que
    // estan incluidos por otro si tienen quien los llame.
    for node in &doc.nodes {
        if node.shape != NodeShape::Ellipse {
            continue;
        }

        let conectado = doc
            .edges
            .iter()
            .any(|e| e.source.node == node.id || e.target.node == node.id);

        if !conectado {
            out.push(
                Diagnostic::warning(
                    "uselessUseCase",
                    format!("A `{}` no llega nadie: ningun actor lo usa.", node.label),
                )
                .at(&node.id),
            );
        }
    }
}

/* --- Secuencia ------------------------------------------------------------------------ */

fn sequence(doc: &Document, out: &mut Vec<Diagnostic>) {
    let Some(script) = &doc.script else { return };

    let known: HashSet<&str> = doc.nodes.iter().map(|n| n.id.as_str()).collect();

    for step in script.walk() {
        match step {
            Step::Message(message) => {
                for (quien, id) in [("origen", &message.from), ("destino", &message.to)] {
                    if !known.contains(id.as_str()) {
                        out.push(
                            Diagnostic::error(
                                "unknownParticipant",
                                format!(
                                    "El mensaje `{}` tiene un {quien} que ya no existe.",
                                    message.text
                                ),
                            )
                            .at(&message.id),
                        );
                    }
                }
            }
            Step::Block(block) => {
                if block.branches.is_empty() {
                    out.push(
                        Diagnostic::warning(
                            "emptyBlock",
                            format!("El bloque `{}` no tiene ninguna rama.", block.kind.tag()),
                        )
                        .at(&block.id),
                    );
                }

                for branch in &block.branches {
                    if branch.steps.is_empty() {
                        out.push(
                            Diagnostic::warning(
                                "emptyBranch",
                                format!(
                                    "La rama `{}` de `{}` esta vacia.",
                                    branch.label,
                                    block.kind.tag()
                                ),
                            )
                            .at(&branch.id),
                        );
                    }
                }
            }
            Step::Note(note) => {
                for id in &note.participants {
                    if !known.contains(id.as_str()) {
                        out.push(
                            Diagnostic::error(
                                "unknownParticipant",
                                "La nota apunta a un participante que ya no existe.",
                            )
                            .at(&note.id),
                        );
                    }
                }
            }
        }
    }

    // Una activacion que se cierra sin haberse abierto es un desajuste del
    // guion, y la barra de activacion sale donde no toca.
    let mut depth: HashMap<&str, i32> = HashMap::new();
    for message in script.messages() {
        if message.activate {
            *depth.entry(message.to.as_str()).or_default() += 1;
        }
        if message.deactivate {
            let open = depth.entry(message.from.as_str()).or_default();
            if *open == 0 {
                out.push(
                    Diagnostic::warning(
                        "unbalancedActivation",
                        format!(
                            "El mensaje `{}` cierra una activacion que no estaba abierta.",
                            message.text
                        ),
                    )
                    .at(&message.id),
                );
            } else {
                *open -= 1;
            }
        }
    }
}

/* --- Alcanzabilidad --------------------------------------------------------------------- */

/// Avisa de los nodos a los que no se llega desde ningun arranque.
///
/// Solo se mira si hay arranque: sin el, todo seria inalcanzable y el aviso
/// no diria nada que no diga ya `noStart`.
fn unreachable(doc: &Document, out: &mut Vec<Diagnostic>, code: &str, what: &str) {
    let starts: Vec<&str> = doc
        .nodes
        .iter()
        .filter(|n| n.shape == NodeShape::InitialNode)
        .map(|n| n.id.as_str())
        .collect();

    if starts.is_empty() {
        return;
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack = starts;

    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        for edge in &doc.edges {
            if edge.source.node == id {
                stack.push(edge.target.node.as_str());
            }
        }
    }

    for node in &doc.nodes {
        // Un contenedor o una anotacion no son parte del flujo.
        if matches!(
            node.shape,
            NodeShape::Package | NodeShape::Note | NodeShape::Text
        ) {
            continue;
        }

        if !seen.contains(node.id.as_str()) {
            out.push(
                Diagnostic::warning(
                    code,
                    format!("Al {what} `{}` no se llega desde el arranque.", node.label),
                )
                .at(&node.id),
            );
        }
    }
}

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use super::diagnostic::{Diagnostic, Report};
use super::document::Document;

/// Comprueba las invariantes estructurales del documento.
///
/// No opina sobre si el diagrama tiene sentido para su dominio (eso llega con
/// cada tipo en F3-F6); solo sobre si el grafo es coherente: identificadores
/// unicos, referencias que existen y anidamiento sin ciclos.
pub fn validate(doc: &Document) -> Report {
    let mut out = Vec::new();

    let node_ids = collect_duplicate_ids(doc.nodes.iter().map(|n| n.id.as_str()), &mut out, "nodo");
    collect_duplicate_ids(doc.edges.iter().map(|e| e.id.as_str()), &mut out, "arista");

    check_empty_ids(doc, &mut out);
    check_edge_endpoints(doc, &node_ids, &mut out);
    check_edge_ports(doc, &mut out);
    check_parents(doc, &node_ids, &mut out);
    check_parent_cycles(doc, &mut out);
    check_er_columns(doc, &mut out);

    Report::new(out)
}

fn collect_duplicate_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    out: &mut Vec<Diagnostic>,
    what: &str,
) -> HashSet<&'a str> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            out.push(
                Diagnostic::error(
                    "duplicateId",
                    format!("Hay mas de un {what} con id `{id}`."),
                )
                .at(id),
            );
        }
    }
    seen
}

fn check_empty_ids(doc: &Document, out: &mut Vec<Diagnostic>) {
    for n in &doc.nodes {
        if n.id.trim().is_empty() {
            out.push(Diagnostic::error(
                "emptyId",
                format!("El nodo etiquetado `{}` no tiene id.", n.label),
            ));
        }
    }
    for e in &doc.edges {
        if e.id.trim().is_empty() {
            out.push(Diagnostic::error(
                "emptyId",
                format!(
                    "La arista entre `{}` y `{}` no tiene id.",
                    e.source.node, e.target.node
                ),
            ));
        }
    }
}

fn check_edge_endpoints(doc: &Document, node_ids: &HashSet<&str>, out: &mut Vec<Diagnostic>) {
    for e in &doc.edges {
        for (extremo, endpoint) in [("origen", &e.source), ("destino", &e.target)] {
            if !node_ids.contains(endpoint.node.as_str()) {
                out.push(
                    Diagnostic::error(
                        "danglingEdge",
                        format!(
                            "La arista `{}` apunta a un nodo inexistente como {extremo}: `{}`.",
                            e.id, endpoint.node
                        ),
                    )
                    .at(&e.id),
                );
            }
        }
    }
}

fn check_parents(doc: &Document, node_ids: &HashSet<&str>, out: &mut Vec<Diagnostic>) {
    for n in &doc.nodes {
        let Some(parent) = n.parent.as_deref() else {
            continue;
        };

        if parent == n.id {
            out.push(
                Diagnostic::error(
                    "selfParent",
                    format!("El nodo `{}` se declara contenedor de si mismo.", n.id),
                )
                .at(&n.id),
            );
        } else if !node_ids.contains(parent) {
            out.push(
                Diagnostic::error(
                    "missingParent",
                    format!(
                        "El nodo `{}` declara como contenedor a `{parent}`, que no existe.",
                        n.id
                    ),
                )
                .at(&n.id),
            );
        }
    }
}

/// Las aristas que apuntan a una columna concreta tienen que apuntar a una
/// que exista. Una referencia rota aqui produce codigo generado sin sentido.
fn check_edge_ports(doc: &Document, out: &mut Vec<Diagnostic>) {
    for edge in &doc.edges {
        for (extremo, endpoint) in [("origen", &edge.source), ("destino", &edge.target)] {
            let Some(port) = endpoint.port.as_deref() else {
                continue;
            };
            let Some(node) = doc.node(&endpoint.node) else {
                // `check_edge_endpoints` ya reporto que el nodo no existe.
                continue;
            };

            let exists = node.er().is_some_and(|t| t.column(port).is_some());
            if !exists {
                out.push(
                    Diagnostic::error(
                        "unknownPort",
                        format!(
                            "La arista `{}` se ancla en su {extremo} a una columna que no existe.",
                            edge.id
                        ),
                    )
                    .at(&edge.id),
                );
            }
        }
    }
}

/// Avisa de lo que impediria exportar un entidad-relacion a DBML.
///
/// Son advertencias y no errores: el diagrama se dibuja igual, y obligar a
/// rellenar un tipo para poder seguir dibujando seria estorbar.
fn check_er_columns(doc: &Document, out: &mut Vec<Diagnostic>) {
    for node in &doc.nodes {
        let Some(table) = node.er() else { continue };

        for column in &table.columns {
            if column.data_type.trim().is_empty() {
                out.push(
                    Diagnostic::warning(
                        "columnWithoutType",
                        format!(
                            "La columna `{}` de `{}` no tiene tipo; dbdiagram.io lo exige.",
                            column.name, node.label
                        ),
                    )
                    .at(&node.id),
                );
            }
        }
    }
}

/// Detecta ciclos de anidamiento (`a` dentro de `b` dentro de `a`). Sin esto,
/// el auto-layout jerarquico se cuelga.
///
/// El caso trivial `a` dentro de `a` se excluye a proposito: `check_parents`
/// ya lo reporta como `selfParent`, que dice mas, y asi no salen dos
/// diagnosticos por el mismo error.
fn check_parent_cycles(doc: &Document, out: &mut Vec<Diagnostic>) {
    let parents: HashMap<&str, &str> = doc
        .nodes
        .iter()
        .filter_map(|n| n.parent.as_deref().map(|p| (n.id.as_str(), p)))
        .filter(|(id, parent)| id != parent)
        .collect();

    let mut reported: HashSet<&str> = HashSet::new();

    for &start in parents.keys() {
        if reported.contains(start) {
            continue;
        }

        let mut path = vec![start];
        let mut cursor = start;

        while let Some(&parent) = parents.get(cursor) {
            if path.contains(&parent) {
                let ciclo = path
                    .iter()
                    .skip_while(|&&id| id != parent)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" -> ");

                reported.extend(&path);
                out.push(
                    Diagnostic::error(
                        "parentCycle",
                        format!("Anidamiento circular: {ciclo} -> {parent}."),
                    )
                    .at(parent),
                );
                break;
            }

            path.push(parent);
            cursor = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::DiagramKind;
    use crate::model::edge::{Edge, EdgeKind};
    use crate::model::node::{Node, NodeShape};

    fn doc() -> Document {
        Document::new("d1", DiagramKind::Architecture, 0)
    }

    fn rect(id: &str) -> Node {
        Node::new(id, id, NodeShape::Rectangle)
    }

    fn codes(report: &Report) -> Vec<&str> {
        report.diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    #[test]
    fn un_documento_vacio_es_valido() {
        assert!(validate(&doc()).ok);
    }

    #[test]
    fn un_grafo_bien_formado_es_valido() {
        let mut d = doc();
        d.nodes.extend([rect("a"), rect("b")]);
        d.edges
            .push(Edge::new("e1", "a", "b", EdgeKind::Association));

        let r = validate(&d);
        assert!(r.ok, "diagnosticos inesperados: {:?}", r.diagnostics);
    }

    #[test]
    fn detecta_ids_de_nodo_repetidos() {
        let mut d = doc();
        d.nodes.extend([rect("a"), rect("a")]);

        assert_eq!(codes(&validate(&d)), ["duplicateId"]);
    }

    #[test]
    fn detecta_ids_de_arista_repetidos() {
        let mut d = doc();
        d.nodes.extend([rect("a"), rect("b")]);
        d.edges.extend([
            Edge::new("e1", "a", "b", EdgeKind::Association),
            Edge::new("e1", "b", "a", EdgeKind::Association),
        ]);

        assert_eq!(codes(&validate(&d)), ["duplicateId"]);
    }

    #[test]
    fn detecta_aristas_colgando_en_ambos_extremos() {
        let mut d = doc();
        d.nodes.push(rect("a"));
        d.edges
            .push(Edge::new("e1", "fantasma", "otro", EdgeKind::Flow));

        assert_eq!(codes(&validate(&d)), ["danglingEdge", "danglingEdge"]);
    }

    #[test]
    fn detecta_un_contenedor_inexistente() {
        let mut d = doc();
        d.nodes.push(rect("a").with_parent("pkg"));

        assert_eq!(codes(&validate(&d)), ["missingParent"]);
    }

    #[test]
    fn detecta_un_nodo_contenedor_de_si_mismo() {
        let mut d = doc();
        d.nodes.push(rect("a").with_parent("a"));

        assert_eq!(codes(&validate(&d)), ["selfParent"]);
    }

    #[test]
    fn detecta_un_ciclo_de_anidamiento_de_dos() {
        let mut d = doc();
        d.nodes
            .extend([rect("a").with_parent("b"), rect("b").with_parent("a")]);

        assert_eq!(codes(&validate(&d)), ["parentCycle"]);
    }

    #[test]
    fn un_ciclo_largo_se_reporta_una_sola_vez() {
        let mut d = doc();
        d.nodes.extend([
            rect("a").with_parent("b"),
            rect("b").with_parent("c"),
            rect("c").with_parent("a"),
        ]);

        assert_eq!(codes(&validate(&d)), ["parentCycle"]);
    }

    #[test]
    fn el_anidamiento_profundo_sin_ciclo_es_valido() {
        let mut d = doc();
        d.nodes.extend([
            rect("raiz"),
            rect("medio").with_parent("raiz"),
            rect("hoja").with_parent("medio"),
        ]);

        let r = validate(&d);
        assert!(r.ok, "diagnosticos inesperados: {:?}", r.diagnostics);
    }

    #[test]
    fn detecta_una_arista_anclada_a_una_columna_inexistente() {
        use crate::model::{ErColumn, ErTable};

        let mut d = doc();
        d.nodes.push(Node::table(
            "t",
            "users",
            ErTable {
                columns: vec![ErColumn::new("t.id", "id").with_type("integer")],
                ..Default::default()
            },
        ));
        let mut edge = Edge::new("e1", "t", "t", EdgeKind::Association);
        edge.source.port = Some("t.fantasma".into());
        d.edges.push(edge);

        assert_eq!(codes(&validate(&d)), ["unknownPort"]);
    }

    #[test]
    fn una_columna_sin_tipo_es_una_advertencia_no_un_error() {
        use crate::model::{ErColumn, ErTable};

        let mut d = doc();
        d.nodes.push(Node::table(
            "t",
            "users",
            ErTable {
                columns: vec![ErColumn::new("t.a", "sin_tipo")],
                ..Default::default()
            },
        ));

        let report = validate(&d);

        // Dibujar sigue siendo posible; solo la exportacion se resentiria.
        assert!(report.ok);
        assert_eq!(codes(&report), ["columnWithoutType"]);
    }

    #[test]
    fn una_tabla_con_todas_sus_columnas_tipadas_no_avisa() {
        use crate::model::{ErColumn, ErTable};

        let mut d = doc();
        d.nodes.push(Node::table(
            "t",
            "users",
            ErTable {
                columns: vec![ErColumn::new("t.a", "id").with_type("integer")],
                ..Default::default()
            },
        ));

        assert!(validate(&d).diagnostics.is_empty());
    }

    #[test]
    fn un_bucle_sobre_si_mismo_es_una_arista_valida() {
        let mut d = doc();
        d.nodes.push(rect("a"));
        d.edges.push(Edge::new("e1", "a", "a", EdgeKind::Flow));

        assert!(validate(&d).ok);
    }
}

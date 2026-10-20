//! Generador de `flowchart` de Mermaid.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{DiagramKind, Document, LayoutDirection, NodeShape};
use crate::parse::mermaid::{arrow_for, delimiters_for};

pub mod class_diagram;
pub mod sequence;

/// Sangria de cada nivel de `subgraph`.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    // Cada tipo de diagrama es otro dialecto entero: otra cabecera, otras
    // flechas y, en el caso de la secuencia, un arbol en vez de una lista de
    // aristas.
    if doc.kind.is_lane_based() {
        return sequence::emit(doc);
    }
    if doc.kind == DiagramKind::Class {
        return class_diagram::emit(doc);
    }

    let mut out = String::new();
    let _ = writeln!(out, "flowchart {}", direction_code(doc.direction));

    // Identificadores cortos y estables: `n1`, `n2`... Los del documento
    // llevan prefijos internos que en el codigo solo serian ruido.
    let names = short_names(doc);

    emit_level(doc, None, 1, &names, &mut out);
    emit_edges(doc, &names, &mut out);
    emit_styles(doc, &names, &mut out);

    out
}

fn direction_code(direction: LayoutDirection) -> &'static str {
    match direction {
        LayoutDirection::Right => "LR",
        LayoutDirection::Left => "RL",
        LayoutDirection::Down => "TD",
        LayoutDirection::Up => "BT",
    }
}

/// Nombre corto de cada nodo, por id.
fn short_names(doc: &Document) -> HashMap<String, String> {
    doc.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("n{}", index + 1)))
        .collect()
}

/// Escribe los nodos de un nivel y, recursivamente, los de sus contenedores.
fn emit_level(
    doc: &Document,
    parent: Option<&str>,
    depth: usize,
    names: &HashMap<String, String>,
    out: &mut String,
) {
    let indent = INDENT.repeat(depth);

    for node in doc.children_of(parent) {
        let name = &names[&node.id];

        if node.shape == NodeShape::Package {
            let _ = writeln!(out, "{indent}subgraph {name}[{}]", escape(&node.label));
            emit_level(doc, Some(&node.id), depth + 1, names, out);
            let _ = writeln!(out, "{indent}end");
            continue;
        }

        let (open, close) = delimiters_for(node.shape);
        let _ = writeln!(out, "{indent}{name}{open}{}{close}", escape(&node.label));
    }
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

        let arrow = arrow_for(
            edge.kind,
            edge.style
                .dashed
                .unwrap_or_else(|| edge.kind.is_dashed_by_default()),
            edge.style.stroke_width.is_some_and(|w| w >= 2.0),
        );

        match &edge.label {
            Some(label) if !label.trim().is_empty() => {
                let _ = writeln!(out, "{INDENT}{from} {arrow}|{}| {to}", escape(label));
            }
            _ => {
                let _ = writeln!(out, "{INDENT}{from} {arrow} {to}");
            }
        }
    }
}

fn emit_styles(doc: &Document, names: &HashMap<String, String>, out: &mut String) {
    let mut wrote_any = false;

    for node in &doc.nodes {
        let mut pairs: Vec<String> = Vec::new();
        if let Some(fill) = &node.style.fill {
            pairs.push(format!("fill:{fill}"));
        }
        if let Some(stroke) = &node.style.stroke {
            pairs.push(format!("stroke:{stroke}"));
        }
        if let Some(color) = &node.style.text_color {
            pairs.push(format!("color:{color}"));
        }

        if pairs.is_empty() {
            continue;
        }

        if !wrote_any {
            out.push('\n');
            wrote_any = true;
        }
        let _ = writeln!(out, "{INDENT}style {} {}", names[&node.id], pairs.join(","));
    }
}

/// Prepara una etiqueta para ir dentro de los delimitadores de una forma.
///
/// Se entrecomilla en cuanto lleva algo que Mermaid interpretaria —un
/// corchete, una coma, un parentesis— y los saltos de linea se escriben con
/// la etiqueta HTML, que es como los entiende.
fn escape(label: &str) -> String {
    let single_line = label.replace('\n', "<br/>");

    let needs_quotes = single_line
        .chars()
        .any(|c| "[]{}()<>|,;\"#".contains(c) || c == '-');

    if needs_quotes {
        format!("\"{}\"", single_line.replace('"', "'"))
    } else {
        single_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::mermaid as parser;

    /// Importa, genera y vuelve a importar.
    fn round_trip(source: &str) -> (Document, Document) {
        let first = parser::import(source, 0);
        assert!(
            first.ok(),
            "la primera importacion fallo: {:#?}",
            first.diagnostics
        );

        let code = emit(&first.document);
        let second = parser::import(&code, 0);
        assert!(
            second.ok(),
            "el codigo generado no vuelve a entrar:\n{code}\n{:#?}",
            second.diagnostics
        );

        (first.document, second.document)
    }

    fn assert_equivalent(a: &Document, b: &Document) {
        assert_eq!(a.direction, b.direction, "distinta direccion");
        assert_eq!(a.nodes.len(), b.nodes.len(), "distinto numero de nodos");
        assert_eq!(a.edges.len(), b.edges.len(), "distinto numero de aristas");

        for (left, right) in a.nodes.iter().zip(&b.nodes) {
            assert_eq!(left.label, right.label);
            assert_eq!(left.shape, right.shape, "forma de `{}`", left.label);
            assert_eq!(left.style.fill, right.style.fill);
        }

        for (left, right) in a.edges.iter().zip(&b.edges) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.label, right.label);
            assert_eq!(left.style.dashed, right.style.dashed);
        }
    }

    #[test]
    fn un_grafo_simple_sobrevive_la_ida_y_vuelta() {
        let (a, b) = round_trip("flowchart LR\n  a[Web] --> b[(Base)]");

        assert_equivalent(&a, &b);
    }

    #[test]
    fn el_codigo_generado_es_legible() {
        let imported = parser::import("flowchart LR\n  a[Web] --> b[API]", 0);

        assert_eq!(
            emit(&imported.document),
            "flowchart LR\n  n1[Web]\n  n2[API]\n\n  n1 --> n2\n"
        );
    }

    #[test]
    fn conserva_la_direccion() {
        for code in ["TD", "BT", "LR", "RL"] {
            let (a, b) = round_trip(&format!("flowchart {code}\n  a --> b"));
            assert_equivalent(&a, &b);
        }
    }

    #[test]
    fn conserva_todas_las_formas() {
        let (a, b) = round_trip(
            "flowchart TD\n\
             a[Rect]\n b(Redondo)\n c[(Cilindro)]\n d((Circulo))\n\
             e{Rombo}\n f{{Hexagono}}\n g[/Paralelo/]",
        );

        assert_equivalent(&a, &b);
    }

    #[test]
    fn conserva_los_tipos_de_flecha() {
        let (a, b) =
            round_trip("flowchart LR\n  a --> b\n  c --- d\n  e -.-> f\n  g -.- h\n  i ==> j");

        assert_equivalent(&a, &b);
    }

    #[test]
    fn conserva_las_etiquetas_de_las_flechas() {
        let (_, b) = round_trip("flowchart LR\n  a -->|consulta| b\n  c -- envia --> d");

        assert_eq!(b.edges[0].label.as_deref(), Some("consulta"));
        assert_eq!(b.edges[1].label.as_deref(), Some("envia"));
    }

    #[test]
    fn conserva_los_subgraph_anidados() {
        let (a, b) = round_trip(
            "flowchart TD\n\
             subgraph fuera[Fuera]\n\
               subgraph dentro[Dentro]\n\
                 a[Hoja]\n\
               end\n\
             end",
        );

        assert_equivalent(&a, &b);
        let dentro = b.nodes.iter().find(|n| n.label == "Dentro").unwrap();
        assert_eq!(b.children_of(Some(&dentro.id)).count(), 1);
    }

    #[test]
    fn conserva_los_colores() {
        let (a, b) = round_trip("flowchart TD\n  a[Web]\n  style a fill:#f9f");

        assert_equivalent(&a, &b);
        assert_eq!(b.nodes[0].style.fill.as_deref(), Some("#f9f"));
    }

    #[test]
    fn entrecomilla_las_etiquetas_con_signos() {
        // Sin comillas, Mermaid leeria los corchetes como parte de la forma.
        let (a, b) = round_trip("flowchart TD\n  a[\"Con, coma y [corchete]\"]");

        assert_equivalent(&a, &b);
        assert_eq!(b.nodes[0].label, "Con, coma y [corchete]");
    }

    #[test]
    fn conserva_los_saltos_de_linea() {
        let (_, b) = round_trip("flowchart TD\n  a[Primera<br/>Segunda]");

        assert_eq!(b.nodes[0].label, "Primera\nSegunda");
    }

    #[test]
    fn un_documento_vacio_genera_solo_la_cabecera() {
        let doc = Document::new("d", crate::model::DiagramKind::Architecture, 0);

        assert_eq!(emit(&doc), "flowchart LR\n");
    }

    #[test]
    fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
        let source = r#"
flowchart LR
  usuario([Cliente])

  subgraph servicios[Servicios]
    api[API REST]
    cola{{Cola}}
  end

  base[(PostgreSQL)]

  usuario --> api
  api --> base
  api -.-> cola
  api ==>|cobro| pagos[/Pagos/]

  style pagos fill:#fde68a
"#;

        let (a, b) = round_trip(source);
        assert_equivalent(&a, &b);
    }
}

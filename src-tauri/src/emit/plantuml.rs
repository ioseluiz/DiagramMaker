//! Generador de diagramas de componentes de PlantUML.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir
//! el mismo diagrama. Las pruebas de ida y vuelta son las que lo sostienen.
//!
//! Dos cosas no sobreviven el viaje, y es por el idioma, no por descuido:
//!
//! - PlantUML solo sabe de dos direcciones de layout, asi que `Left` sale
//!   como `left to right` y `Up` como `top to bottom`.
//! - El color de relleno se escribe pegado al elemento (`#LightBlue`), pero
//!   el del trazo y el del texto no tienen forma simple de escribirse y se
//!   pierden.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{Document, LayoutDirection, NodeShape};
use crate::parse::plantuml::{arrow_for, keyword_for};

pub mod sequence;

/// Sangria de cada nivel de bloque.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("@startuml\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "title {}", doc.meta.title.replace('\n', " "));
    }
    let _ = writeln!(out, "{}\n", direction_line(doc.direction));

    // Alias cortos y estables: `n1`, `n2`... Los ids del documento llevan
    // prefijos internos que en el codigo solo serian ruido.
    let names = short_names(doc);

    emit_level(doc, None, 0, &names, &mut out);
    emit_edges(doc, &names, &mut out);

    out.push_str("@enduml\n");
    out
}

fn direction_line(direction: LayoutDirection) -> &'static str {
    match direction {
        LayoutDirection::Right | LayoutDirection::Left => "left to right direction",
        LayoutDirection::Down | LayoutDirection::Up => "top to bottom direction",
    }
}

fn short_names(doc: &Document) -> HashMap<String, String> {
    doc.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("n{}", index + 1)))
        .collect()
}

/// Escribe los elementos de un nivel y, recursivamente, los de sus bloques.
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
        let keyword = keyword_for(node.shape);
        let color = match &node.style.fill {
            Some(fill) => format!(" #{}", fill.strip_prefix('#').unwrap_or(fill)),
            None => String::new(),
        };

        let head = format!(
            "{indent}{keyword} \"{}\" as {name}{color}",
            escape(&node.label)
        );

        if node.shape == NodeShape::Package {
            let _ = writeln!(out, "{head} {{");
            emit_level(doc, Some(&node.id), depth + 1, names, out);
            let _ = writeln!(out, "{indent}}}");
        } else {
            let _ = writeln!(out, "{head}");
        }
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

        let dashed = edge
            .style
            .dashed
            .unwrap_or_else(|| edge.kind.is_dashed_by_default());

        let _ = write!(out, "{from}{}", multiplicity(&edge.source.label));
        let _ = write!(out, " {}", arrow_for(edge.kind, dashed));
        let _ = write!(out, "{} {to}", multiplicity(&edge.target.label));

        match &edge.label {
            Some(label) if !label.trim().is_empty() => {
                let _ = writeln!(out, " : {}", escape(label));
            }
            _ => out.push('\n'),
        }
    }
}

/// La cardinalidad de un extremo va entrecomillada y pegada a la flecha.
fn multiplicity(label: &Option<String>) -> String {
    match label {
        Some(text) if !text.trim().is_empty() => format!(" \"{}\"", escape(text)),
        _ => String::new(),
    }
}

/// Prepara un texto para ir entre comillas.
///
/// PlantUML escribe los saltos de linea con `\n` dentro de la cadena, y no
/// admite comillas sin escapar: se cambian por simples, que es lo que menos
/// altera el texto.
fn escape(text: &str) -> String {
    text.replace('"', "'").replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramKind, EdgeKind};
    use crate::parse::plantuml as parser;

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
        assert_eq!(a.nodes.len(), b.nodes.len(), "distinto numero de elementos");
        assert_eq!(
            a.edges.len(),
            b.edges.len(),
            "distinto numero de relaciones"
        );

        for (left, right) in a.nodes.iter().zip(&b.nodes) {
            assert_eq!(left.label, right.label);
            assert_eq!(left.shape, right.shape, "forma de `{}`", left.label);
            assert_eq!(
                left.style.fill, right.style.fill,
                "color de `{}`",
                left.label
            );
        }

        for (left, right) in a.edges.iter().zip(&b.edges) {
            assert_eq!(left.kind, right.kind, "tipo de relacion");
            assert_eq!(left.label, right.label);
            assert_eq!(left.source.label, right.source.label);
            assert_eq!(left.target.label, right.target.label);
            assert_eq!(left.style.dashed, right.style.dashed);
        }
    }

    #[test]
    fn un_diagrama_simple_sobrevive_la_ida_y_vuelta() {
        let (a, b) = round_trip("@startuml\n[Portal] --> [API]\n@enduml");

        assert_equivalent(&a, &b);
    }

    #[test]
    fn el_codigo_generado_es_legible() {
        let imported = parser::import("@startuml\n[Portal] --> [API]\n@enduml", 0);

        assert_eq!(
            emit(&imported.document),
            "@startuml\n\
             left to right direction\n\
             \n\
             rectangle \"Portal\" as n1\n\
             rectangle \"API\" as n2\n\
             \n\
             n1 --> n2\n\
             @enduml\n"
        );
    }

    #[test]
    fn conserva_todas_las_formas_que_plantuml_sabe_escribir() {
        let (a, b) = round_trip(
            "@startuml\n\
             component \"Caja\" as a\n\
             card \"Tarjeta\" as b\n\
             database \"Base\" as c\n\
             queue \"Cola\" as d\n\
             usecase \"Caso\" as e\n\
             actor \"Persona\" as f\n\
             @enduml",
        );

        assert_equivalent(&a, &b);
    }

    #[test]
    fn conserva_los_tipos_de_relacion() {
        let (a, b) = round_trip(
            "@startuml\n\
             a --> b\n\
             c -- d\n\
             e ..> f\n\
             g .. h\n\
             i --|> j\n\
             k ..|> l\n\
             m o-- n\n\
             o *-- p\n\
             @enduml",
        );

        assert_equivalent(&a, &b);
        let kinds: Vec<_> = b.edges.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                EdgeKind::Directed,
                EdgeKind::Association,
                EdgeKind::Dependency,
                EdgeKind::Association,
                EdgeKind::Inheritance,
                EdgeKind::Realization,
                EdgeKind::Aggregation,
                EdgeKind::Composition,
            ]
        );
    }

    #[test]
    fn una_flecha_dirigida_punteada_no_se_convierte_en_dependencia() {
        // `..>` significa dependencia en UML. Una flecha corriente dibujada
        // con trazo discontinuo es otra cosa, y se escribe con el estilo.
        let (a, b) = round_trip(
            "@startuml
[A] -[dashed]-> [B]
@enduml",
        );

        assert_equivalent(&a, &b);
        assert_eq!(b.edges[0].kind, EdgeKind::Directed);
        assert_eq!(b.edges[0].style.dashed, Some(true));
    }

    #[test]
    fn conserva_las_etiquetas_y_las_cardinalidades() {
        let (a, b) = round_trip("@startuml\napi \"1\" --> \"n\" base : consulta\n@enduml");

        assert_equivalent(&a, &b);
        assert_eq!(b.edges[0].label.as_deref(), Some("consulta"));
        assert_eq!(b.edges[0].source.label.as_deref(), Some("1"));
        assert_eq!(b.edges[0].target.label.as_deref(), Some("n"));
    }

    #[test]
    fn conserva_los_paquetes_anidados() {
        let (a, b) = round_trip(
            "@startuml\n\
             package \"Fuera\" {\n\
               package \"Dentro\" {\n\
                 [Hoja]\n\
               }\n\
             }\n\
             @enduml",
        );

        assert_equivalent(&a, &b);
        let dentro = b.nodes.iter().find(|n| n.label == "Dentro").unwrap();
        assert_eq!(b.children_of(Some(&dentro.id)).count(), 1);
    }

    #[test]
    fn conserva_el_color_de_relleno() {
        let (a, b) = round_trip("@startuml\ncomponent \"Caja\" as c #LightBlue\n@enduml");

        assert_equivalent(&a, &b);
        assert_eq!(b.nodes[0].style.fill.as_deref(), Some("LightBlue"));
    }

    #[test]
    fn conserva_un_color_hexadecimal() {
        let (a, b) = round_trip("@startuml\ncomponent \"Caja\" as c #AACCFF\n@enduml");

        assert_equivalent(&a, &b);
        assert_eq!(b.nodes[0].style.fill.as_deref(), Some("#AACCFF"));
    }

    #[test]
    fn conserva_las_dos_direcciones_que_plantuml_tiene() {
        for (line, expected) in [
            ("left to right direction", LayoutDirection::Right),
            ("top to bottom direction", LayoutDirection::Down),
        ] {
            let (a, b) = round_trip(&format!("@startuml\n{line}\n[A] --> [B]\n@enduml"));
            assert_equivalent(&a, &b);
            assert_eq!(b.direction, expected);
        }
    }

    #[test]
    fn conserva_el_titulo() {
        let (a, b) = round_trip("@startuml\ntitle Mi arquitectura\n[A]\n@enduml");

        assert_equivalent(&a, &b);
        assert_eq!(b.meta.title, "Mi arquitectura");
    }

    #[test]
    fn una_etiqueta_con_una_flecha_dentro_no_parte_la_linea() {
        // Al volver a leerse, las comillas tienen que proteger la flecha.
        let (_, b) = round_trip("@startuml\ncomponent \"entrada --> salida\" as a\n@enduml");

        assert_eq!(b.nodes.len(), 1);
        assert_eq!(b.nodes[0].label, "entrada --> salida");
    }

    #[test]
    fn conserva_los_saltos_de_linea_de_una_etiqueta() {
        let (_, b) = round_trip("@startuml\n[A] --> [B] : primera\\nsegunda\n@enduml");

        assert_eq!(b.edges[0].label.as_deref(), Some("primera\nsegunda"));
    }

    #[test]
    fn un_documento_vacio_genera_solo_el_armazon() {
        let doc = Document::new("d", DiagramKind::Architecture, 0);

        assert_eq!(
            emit(&doc),
            "@startuml\nleft to right direction\n\n@enduml\n"
        );
    }

    #[test]
    fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
        let source = r#"
@startuml
title Plataforma de pagos
left to right direction

actor "Cliente" as cliente

package "Capa publica" {
  [Portal web] as web
  component "API REST" as api
}

package "Datos" {
  database "PostgreSQL" as base #AACCFF
  queue "Eventos" as cola
}

cliente --> web : HTTPS
web --> api
api --> base
api ..> cola : publica
@enduml
"#;

        let (a, b) = round_trip(source);
        assert_equivalent(&a, &b);
    }
}

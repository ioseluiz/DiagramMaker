use super::*;
use crate::model::{DiagramKind, Visibility};
use crate::parse::mermaid::class_diagram as parser;

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

/// Resumen de un documento, sin ids: es lo que tiene que coincidir.
fn shape_of(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| {
            let empty = ClassBox::new();
            let box_ = node.class().unwrap_or(&empty);
            format!(
                "{} {:?} <<{}>> [{}] [{}]",
                node.label,
                node.shape,
                box_.stereotype.clone().unwrap_or_default(),
                box_.attributes
                    .iter()
                    .map(|m| member_text(m, false))
                    .collect::<Vec<_>>()
                    .join(", "),
                box_.operations
                    .iter()
                    .map(|m| member_text(m, true))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
        .collect();

    for edge in &doc.edges {
        out.push(format!(
            "{} {:?}{} {}{}",
            doc.node(&edge.source.node).map_or("?", |n| &n.label),
            edge.kind,
            edge.source.label.clone().unwrap_or_default(),
            doc.node(&edge.target.node).map_or("?", |n| &n.label),
            edge.target.label.clone().unwrap_or_default(),
        ));
    }

    out
}

fn assert_equivalent(a: &Document, b: &Document) {
    assert_eq!(a.direction, b.direction, "distinta direccion");
    assert_eq!(shape_of(a), shape_of(b), "distinto contenido");
}

#[test]
fn una_clase_con_miembros_sobrevive_la_ida_y_vuelta() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 class Pedido {\n\
         \x20   -int id\n\
         \x20   +confirmar()\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("classDiagram\n  class Pedido {\n    -int id\n  }", 0);

    assert_eq!(
        emit(&imported.document),
        "classDiagram\n\
         \x20 direction LR\n\
         \x20 class Pedido {\n\
         \x20   -int id\n\
         \x20 }\n"
    );
}

#[test]
fn los_nombres_no_se_acortan() {
    // En un diagrama de clases el nombre de la clase es el nombre del tipo;
    // leer `n3 <|-- n7` no diria nada.
    let imported = parser::import("classDiagram\n  Pedido <|-- PedidoEspecial", 0);
    let code = emit(&imported.document);

    assert!(code.contains("Pedido"), "{code}");
    assert!(code.contains("PedidoEspecial"), "{code}");
    assert!(!code.contains("n1"), "{code}");
}

#[test]
fn una_etiqueta_con_espacios_se_sanea_y_se_conserva() {
    let (a, b) = round_trip("classDiagram\n  class Pedido[\"Pedido de compra\"]");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "Pedido de compra");
}

#[test]
fn dos_etiquetas_que_se_sanean_igual_no_colisionan() {
    let imported = parser::import(
        "classDiagram\n  class A[\"Mi clase\"]\n  class B[\"Mi-clase\"]",
        0,
    );
    let code = emit(&imported.document);
    let second = parser::import(&code, 0);

    assert!(second.ok(), "{code}\n{:#?}", second.diagnostics);
    assert_eq!(second.document.nodes.len(), 2, "{code}");
}

#[test]
fn conserva_las_cuatro_visibilidades() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 class Pedido {\n\
         \x20   +publico\n\
         \x20   -privado\n\
         \x20   #protegido\n\
         \x20   ~paquete\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
    let visibilities: Vec<_> = b.nodes[0]
        .class()
        .unwrap()
        .attributes
        .iter()
        .map(|m| m.visibility)
        .collect();
    assert_eq!(
        visibilities,
        [
            Visibility::Public,
            Visibility::Private,
            Visibility::Protected,
            Visibility::Package,
        ]
    );
}

#[test]
fn conserva_lo_estatico_y_lo_abstracto() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 class Pedido {\n\
         \x20   +int cuenta$\n\
         \x20   +calcular()*\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
    assert!(b.nodes[0].class().unwrap().attributes[0].is_static);
    assert!(b.nodes[0].class().unwrap().operations[0].is_abstract);
}

#[test]
fn conserva_los_parametros_y_lo_que_devuelve_una_operacion() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 class Pedido {\n\
         \x20   +buscar(int id, bool activo) Pedido\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
    let operation = &b.nodes[0].class().unwrap().operations[0];
    assert_eq!(operation.parameters, "int id, bool activo");
    assert_eq!(operation.type_name, "Pedido");
}

#[test]
fn conserva_el_estereotipo() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 class Repositorio {\n\
         \x20   <<interface>>\n\
         \x20   +guardar()\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes[0].class().unwrap().stereotype.as_deref(),
        Some("interface")
    );
}

#[test]
fn un_estereotipo_sin_miembros_sigue_necesitando_bloque() {
    let (a, b) = round_trip("classDiagram\n  class Vacia {\n    <<interface>>\n  }");

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes[0].class().unwrap().stereotype.as_deref(),
        Some("interface")
    );
}

#[test]
fn conserva_los_tipos_de_relacion() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 A <|-- B\n\
         \x20 C *-- D\n\
         \x20 E o-- F\n\
         \x20 G --> H\n\
         \x20 I ..> J\n\
         \x20 K ..|> L\n\
         \x20 M -- N\n\
         \x20 O .. P",
    );

    assert_equivalent(&a, &b);
    let kinds: Vec<_> = b.edges.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            EdgeKind::Inheritance,
            EdgeKind::Composition,
            EdgeKind::Aggregation,
            EdgeKind::Directed,
            EdgeKind::Dependency,
            EdgeKind::Realization,
            EdgeKind::Association,
            EdgeKind::Association,
        ]
    );
}

#[test]
fn la_herencia_sale_con_la_punta_a_la_derecha() {
    // Escribirla siempre igual evita tener que invertir nada al releerla.
    let imported = parser::import("classDiagram\n  Padre <|-- Hijo", 0);
    let code = emit(&imported.document);

    assert!(code.contains("Hijo --|> Padre"), "{code}");
}

#[test]
fn conserva_las_cardinalidades_y_la_etiqueta() {
    let (a, b) = round_trip("classDiagram\n  Pedido \"1\" *-- \"0..*\" Linea : contiene");

    assert_equivalent(&a, &b);
    assert_eq!(b.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(b.edges[0].target.label.as_deref(), Some("0..*"));
    assert_eq!(b.edges[0].label.as_deref(), Some("contiene"));
}

#[test]
fn conserva_los_namespaces() {
    let (a, b) = round_trip(
        "classDiagram\n\
         \x20 namespace Dominio {\n\
         \x20   class Pedido\n\
         \x20   class Linea\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
    let group = b
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el contenedor");
    assert_eq!(b.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn conserva_la_direccion() {
    for code in ["LR", "RL", "TB", "BT"] {
        let (a, b) = round_trip(&format!("classDiagram\n  direction {code}\n  class A"));
        assert_equivalent(&a, &b);
    }
}

#[test]
fn un_documento_vacio_genera_solo_la_cabecera() {
    let doc = Document::new("d", DiagramKind::Class, 0);

    assert_eq!(emit(&doc), "classDiagram\n  direction LR\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = "classDiagram\n\
         \x20 direction LR\n\
         \n\
         \x20 namespace Dominio {\n\
         \x20   class Pedido {\n\
         \x20     -int id\n\
         \x20     -Decimal total\n\
         \x20     +int creados$\n\
         \x20     +confirmar()\n\
         \x20     +buscar(int id) Pedido\n\
         \x20   }\n\
         \x20   class Linea {\n\
         \x20     -int cantidad\n\
         \x20   }\n\
         \x20 }\n\
         \n\
         \x20 class Repositorio {\n\
         \x20   <<interface>>\n\
         \x20   +guardar(Pedido p)*\n\
         \x20 }\n\
         \n\
         \x20 class PedidoEspecial\n\
         \n\
         \x20 Pedido <|-- PedidoEspecial\n\
         \x20 Pedido \"1\" *-- \"0..*\" Linea : contiene\n\
         \x20 Pedido ..> Repositorio : usa";

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}

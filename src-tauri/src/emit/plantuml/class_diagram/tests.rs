use super::*;
use crate::model::{DiagramKind, EdgeKind, Visibility};
use crate::parse::plantuml::class_diagram as parser;

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

/// Lo mismo, pero entrando por el despachador: comprueba ademas que el codigo
/// generado se sigue reconociendo como un diagrama de clases.
fn round_trip_sniffed(source: &str) -> Document {
    let first = crate::parse::plantuml::import(source, 0);
    assert_eq!(first.document.kind, DiagramKind::Class, "no lo olfateo");

    let code = emit(&first.document);
    let second = crate::parse::plantuml::import(&code, 0);

    assert_eq!(
        second.document.kind,
        DiagramKind::Class,
        "el codigo generado deja de parecer un diagrama de clases:\n{code}"
    );
    second.document
}

/// Resumen de un documento, sin ids.
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
    assert_eq!(shape_of(a), shape_of(b), "distinto contenido");
}

#[test]
fn una_clase_con_miembros_sobrevive_la_ida_y_vuelta() {
    let (a, b) = round_trip(
        "@startuml\n\
         class Pedido {\n\
         \x20 - id : int\n\
         \x20 + confirmar()\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("@startuml\nclass Pedido {\n  - id : int\n}\n@enduml", 0);

    assert_eq!(
        emit(&imported.document),
        "@startuml\n\
         \n\
         class Pedido {\n\
         \x20 - id : int\n\
         }\n\
         @enduml\n"
    );
}

#[test]
fn el_codigo_generado_se_sigue_reconociendo_como_clases() {
    // Es la trampa del olfateador: si lo que sale no llevara ninguna palabra
    // de clases, al volver a entrar se leeria como de componentes. No pasa
    // porque el emisor declara siempre cada clase, aunque este vacia.
    let doc = round_trip_sniffed(
        "@startuml\nclass Pedido\nclass PedidoEspecial\nPedido <|-- PedidoEspecial\n@enduml",
    );

    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.nodes.iter().all(|n| n.shape == NodeShape::Class));
}

#[test]
fn una_relacion_suelta_sin_declarar_nada_se_lee_como_componentes() {
    // Es la ambiguedad que el olfateador deja anotada: `A <|-- B` a secas no
    // tiene ninguna palabra que decida, y se queda con la lectura que ya
    // habia. Degrada suave —salen dos cajas con su relacion— y basta con
    // cambiar el tipo de diagrama en la barra.
    let result = crate::parse::plantuml::import("@startuml\nA <|-- B\n@enduml", 0);

    assert_eq!(result.document.kind, DiagramKind::Architecture);
    assert_eq!(result.document.edges[0].kind, EdgeKind::Inheritance);
}

#[test]
fn cada_estereotipo_vuelve_con_su_palabra() {
    // Un `interface` no sale como `class X <<interface>>`: sale como
    // `interface X`, que es lo que escribiria una persona.
    let (a, b) = round_trip(
        "@startuml\n\
         class Pedido\n\
         interface Repositorio\n\
         abstract class Base\n\
         enum Estado\n\
         annotation Marca\n\
         @enduml",
    );

    assert_equivalent(&a, &b);

    let code = emit(&a);
    assert!(code.contains("interface Repositorio"), "{code}");
    assert!(code.contains("abstract class Base"), "{code}");
    assert!(code.contains("enum Estado"), "{code}");
}

#[test]
fn un_estereotipo_sin_palabra_propia_sale_entre_angulos() {
    let (a, b) = round_trip("@startuml\nclass Pedido <<servicio>>\n@enduml");

    assert_equivalent(&a, &b);
    assert!(emit(&a).contains("<<servicio>>"));
}

#[test]
fn conserva_las_cuatro_visibilidades() {
    let (a, b) = round_trip(
        "@startuml\n\
         class Pedido {\n\
         \x20 + publico\n\
         \x20 - privado\n\
         \x20 # protegido\n\
         \x20 ~ paquete\n\
         }\n\
         @enduml",
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
        "@startuml\n\
         class Pedido {\n\
         \x20 + {static} creados : int\n\
         \x20 + {abstract} calcular()\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    assert!(b.nodes[0].class().unwrap().attributes[0].is_static);
    assert!(b.nodes[0].class().unwrap().operations[0].is_abstract);
}

#[test]
fn conserva_los_parametros_y_lo_que_devuelve_una_operacion() {
    let (a, b) = round_trip(
        "@startuml\n\
         class Pedido {\n\
         \x20 + buscar(id : int, activo : bool) : Pedido\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    let operation = &b.nodes[0].class().unwrap().operations[0];
    assert_eq!(operation.parameters, "id : int, activo : bool");
    assert_eq!(operation.type_name, "Pedido");
}

#[test]
fn el_separador_solo_sale_cuando_hay_las_dos_cosas() {
    let solo_atributos = parser::import("@startuml\nclass A {\n  - id : int\n}\n@enduml", 0);
    let las_dos = parser::import("@startuml\nclass A {\n  - id : int\n  + f()\n}\n@enduml", 0);

    assert!(!emit(&solo_atributos.document).contains("\n  --\n"));
    assert!(emit(&las_dos.document).contains("\n  --\n"));
}

#[test]
fn conserva_los_tipos_de_relacion() {
    let (a, b) = round_trip(
        "@startuml\n\
         A <|-- B\n\
         C *-- D\n\
         E o-- F\n\
         G --> H\n\
         I ..> J\n\
         K ..|> L\n\
         M -- N\n\
         @enduml",
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
        ]
    );
}

#[test]
fn conserva_las_cardinalidades_y_la_etiqueta() {
    let (a, b) = round_trip("@startuml\nPedido \"1\" *-- \"0..*\" Linea : contiene\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(b.edges[0].target.label.as_deref(), Some("0..*"));
    assert_eq!(b.edges[0].label.as_deref(), Some("contiene"));
}

#[test]
fn conserva_los_paquetes() {
    let (a, b) = round_trip(
        "@startuml\n\
         package Dominio {\n\
         \x20 class Pedido\n\
         \x20 class Linea\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    let group = b
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el paquete");
    assert_eq!(b.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn una_etiqueta_con_espacios_se_sanea_y_se_conserva() {
    let (a, b) = round_trip("@startuml\nclass \"Pedido de compra\" as pedido\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "Pedido de compra");
}

#[test]
fn conserva_el_titulo() {
    let (a, b) = round_trip("@startuml\ntitle Dominio\nclass A\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.meta.title, "Dominio");
}

#[test]
fn un_documento_vacio_genera_solo_el_armazon() {
    let doc = Document::new("d", DiagramKind::Class, 0);

    assert_eq!(emit(&doc), "@startuml\n\n@enduml\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = r#"
@startuml
title Dominio de pedidos

package Dominio {
  class Pedido {
    - id : int
    - total : Decimal
    + {static} creados : int
    --
    + confirmar()
    + buscar(id : int) : Pedido
  }

  class Linea {
    - cantidad : int
  }
}

interface Repositorio {
  + {abstract} guardar(p : Pedido)
  + todos() : List
}

abstract class Base

Pedido <|-- PedidoEspecial
Pedido --|> Base
Pedido "1" *-- "0..*" Linea : contiene
Pedido ..> Repositorio : usa
@enduml
"#;

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}

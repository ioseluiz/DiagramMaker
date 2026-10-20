use super::*;
use crate::model::DiagramKind;
use crate::parse::plantuml::state_diagram as parser;

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

/// Lo mismo, pero entrando por el despachador.
fn round_trip_sniffed(source: &str) -> Document {
    let first = crate::parse::plantuml::import(source, 0);
    assert_eq!(first.document.kind, DiagramKind::State, "no lo olfateo");

    let code = emit(&first.document);
    let second = crate::parse::plantuml::import(&code, 0);

    assert_eq!(
        second.document.kind,
        DiagramKind::State,
        "el codigo generado deja de parecer una maquina de estados:\n{code}"
    );
    second.document
}

/// Resumen de un documento, sin ids.
fn shape_of(doc: &Document) -> Vec<String> {
    let name = |id: &str| -> String {
        match doc.node(id) {
            Some(n) if n.label.is_empty() => format!("{:?}", n.shape),
            Some(n) => n.label.replace('\n', " | "),
            None => "?".into(),
        }
    };

    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{} {:?} en {}",
                if node.label.is_empty() {
                    format!("{:?}", node.shape)
                } else {
                    node.label.replace('\n', " | ")
                },
                node.shape,
                node.parent.as_deref().map(name).unwrap_or("raiz".into()),
            )
        })
        .collect();
    out.sort();

    let mut edges: Vec<String> = doc
        .edges
        .iter()
        .map(|edge| {
            format!(
                "{} -> {} : {}",
                name(&edge.source.node),
                name(&edge.target.node),
                edge.label.clone().unwrap_or_default()
            )
        })
        .collect();
    edges.sort();
    out.extend(edges);

    out
}

fn assert_equivalent(a: &Document, b: &Document) {
    assert_eq!(shape_of(a), shape_of(b), "distinto contenido");
}

#[test]
fn una_maquina_simple_sobrevive_la_ida_y_vuelta() {
    let (a, b) = round_trip("@startuml\n[*] --> Nuevo\nNuevo --> [*]\n@enduml");

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("@startuml\n[*] --> Nuevo\nNuevo --> [*]\n@enduml", 0);

    assert_eq!(
        emit(&imported.document),
        "@startuml\n\
         \n\
         [*] --> Nuevo\n\
         Nuevo --> [*]\n\
         @enduml\n"
    );
}

#[test]
fn el_codigo_generado_se_sigue_reconociendo_como_maquina_de_estados() {
    let doc = round_trip_sniffed("@startuml\n[*] --> Nuevo\nNuevo --> [*]\n@enduml");

    assert_eq!(doc.kind, DiagramKind::State);
}

#[test]
fn conserva_una_descripcion() {
    // En PlantUML la descripcion anade un renglon debajo del nombre, no lo
    // sustituye: al escribirla se vuelven a separar.
    let (a, b) = round_trip("@startuml\nstate Nuevo\nNuevo : esperando pago\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "Nuevo\nesperando pago");
    assert!(emit(&a).contains("Nuevo : esperando pago"));
}

#[test]
fn conserva_varias_descripciones() {
    let (a, b) = round_trip("@startuml\nstate A\nA : primera\nA : segunda\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "A\nprimera\nsegunda");
}

#[test]
fn un_estado_con_descripcion_no_necesita_declaracion() {
    // Su propia linea de descripcion ya lo crea.
    let imported = parser::import("@startuml\nstate A\nA : texto\n@enduml", 0);
    let code = emit(&imported.document);

    assert!(!code.contains("state A\n"), "sobra la declaracion:\n{code}");
    assert!(code.contains("A : texto"), "{code}");
}

#[test]
fn conserva_una_etiqueta_con_espacios() {
    let (a, b) = round_trip("@startuml\nstate \"Pendiente de pago\" as p\np --> [*]\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "Pendiente de pago");
}

#[test]
fn conserva_las_barras_y_los_rombos() {
    let (a, b) = round_trip(
        "@startuml\n\
         state f <<fork>>\n\
         state c <<choice>>\n\
         A --> f\n\
         A --> c\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes.iter().filter(|n| n.shape == NodeShape::Bar).count(),
        1
    );
}

#[test]
fn conserva_los_compuestos() {
    let (a, b) = round_trip(
        "@startuml\n\
         state Activo {\n\
         \x20 Nuevo --> Pagado\n\
         }\n\
         [*] --> Activo\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    let compuesto = b
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el compuesto");
    assert_eq!(b.children_of(Some(&compuesto.id)).count(), 2);
}

#[test]
fn una_transicion_a_un_extremo_se_escribe_dentro_de_su_compuesto() {
    let (a, b) = round_trip(
        "@startuml\n\
         [*] --> Fuera\n\
         state Fuera {\n\
         \x20 [*] --> Dentro\n\
         \x20 Dentro --> [*]\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes
            .iter()
            .filter(|n| n.shape == NodeShape::InitialNode)
            .count(),
        2,
        "uno por nivel"
    );
}

#[test]
fn conserva_los_compuestos_anidados() {
    let (a, b) = round_trip(
        "@startuml\n\
         state Fuera {\n\
         \x20 state Dentro {\n\
         \x20   A --> B\n\
         \x20 }\n\
         }\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_las_etiquetas_de_las_transiciones() {
    let (a, b) = round_trip("@startuml\nNuevo --> Pagado : confirmar\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.edges[0].label.as_deref(), Some("confirmar"));
}

#[test]
fn conserva_los_saltos_de_linea_de_una_etiqueta() {
    let (_, b) = round_trip("@startuml\nA --> B : primera\\nsegunda\n@enduml");

    assert_eq!(b.edges[0].label.as_deref(), Some("primera\nsegunda"));
}

#[test]
fn conserva_el_titulo() {
    let (a, b) = round_trip("@startuml\ntitle Ciclo\nA --> B\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.meta.title, "Ciclo");
}

#[test]
fn un_documento_vacio_genera_solo_el_armazon() {
    let doc = Document::new("d", DiagramKind::State, 0);

    assert_eq!(emit(&doc), "@startuml\n\n@enduml\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = r#"
@startuml
title Ciclo del pedido

[*] --> Nuevo
Nuevo : esperando pago
Nuevo --> Pagado : confirmar

state Pagado {
  [*] --> Cobrando
  Cobrando --> Cobrado
  Cobrado --> [*]
}

state reparto <<fork>>
Pagado --> reparto
reparto --> Enviado
reparto --> Facturado

state cierre <<join>>
Enviado --> cierre
Facturado --> cierre
cierre --> [*]
@enduml
"#;

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}

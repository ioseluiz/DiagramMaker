use super::*;

fn parse(source: &str) -> ImportResult {
    import(source, 0)
}

fn ok(source: &str) -> Document {
    let result = parse(source);
    assert!(
        result.ok(),
        "diagnosticos inesperados: {:#?}",
        result.diagnostics
    );
    result.document
}

/// Importa un cuerpo, poniendole el armazon.
fn body(source: &str) -> Document {
    ok(&format!("@startuml\n{source}\n@enduml"))
}

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn node<'a>(doc: &'a Document, label: &str) -> &'a Node {
    doc.nodes
        .iter()
        .find(|n| n.label == label)
        .unwrap_or_else(|| panic!("no hay ningun estado `{label}`"))
}

fn count(doc: &Document, shape: NodeShape) -> usize {
    doc.nodes.iter().filter(|n| n.shape == shape).count()
}

/* --- Estados -------------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_estados_y_llega_sin_geometria() {
    let doc = body("[*] --> Nuevo");

    assert_eq!(doc.kind, DiagramKind::State);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn un_estado_se_dibuja_redondeado() {
    assert_eq!(
        node(&body("state Nuevo"), "Nuevo").shape,
        NodeShape::RoundedRect
    );
}

#[test]
fn mencionar_un_estado_en_una_transicion_lo_crea() {
    assert_eq!(count(&body("Nuevo --> Pagado"), NodeShape::RoundedRect), 2);
}

#[test]
fn un_nombre_entrecomillado_con_alias_sirve_por_los_dos() {
    let doc = body(
        "state \"Pendiente de pago\" as p\n\
         p --> Fin\n\
         \"Pendiente de pago\" --> Otro",
    );

    assert_eq!(doc.nodes[0].label, "Pendiente de pago");
    assert_eq!(doc.edges.len(), 2);
    assert_eq!(doc.edges[0].source.node, doc.edges[1].source.node);
}

#[test]
fn una_descripcion_se_anade_en_otro_renglon() {
    let doc = body("state Nuevo\nNuevo : esperando pago");

    assert_eq!(doc.nodes[0].label, "Nuevo\nesperando pago");
}

#[test]
fn las_descripciones_se_acumulan() {
    // PlantUML las apila; dos lineas son dos renglones.
    let doc = body("state Nuevo\nNuevo : primera\nNuevo : segunda");

    assert_eq!(doc.nodes[0].label, "Nuevo\nprimera\nsegunda");
}

#[test]
fn lee_el_titulo() {
    assert_eq!(
        body("title Ciclo del pedido\nstate A").meta.title,
        "Ciclo del pedido"
    );
}

#[test]
fn se_salta_los_comentarios() {
    assert_eq!(
        body("' nada\nstate A\n/' ni\nesto '/\nstate B").nodes.len(),
        2
    );
}

/* --- Extremos -------------------------------------------------------------- */

#[test]
fn el_corchete_asterisco_es_arranque_o_salida_segun_el_lado() {
    let entra = body("[*] --> Nuevo");
    let sale = body("Nuevo --> [*]");

    assert_eq!(count(&entra, NodeShape::InitialNode), 1);
    assert_eq!(count(&entra, NodeShape::FinalNode), 0);
    assert_eq!(count(&sale, NodeShape::InitialNode), 0);
    assert_eq!(count(&sale, NodeShape::FinalNode), 1);
}

#[test]
fn hay_uno_de_cada_por_nivel_no_uno_por_linea() {
    let doc = body("[*] --> A\n[*] --> B\nA --> [*]\nB --> [*]");

    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 1);
    assert_eq!(doc.edges.len(), 4);
}

#[test]
fn cada_nivel_tiene_los_suyos() {
    let doc = body(
        "[*] --> Fuera\n\
         state Fuera {\n\
         \x20 [*] --> Dentro\n\
         \x20 Dentro --> [*]\n\
         }\n\
         Fuera --> [*]",
    );

    assert_eq!(count(&doc, NodeShape::InitialNode), 2);
    assert_eq!(count(&doc, NodeShape::FinalNode), 2);
}

/* --- Estereotipos ---------------------------------------------------------- */

#[test]
fn fork_y_join_se_dibujan_como_barras() {
    assert_eq!(
        count(&body("state f <<fork>>\nstate j <<join>>"), NodeShape::Bar),
        2
    );
}

#[test]
fn choice_se_dibuja_como_rombo() {
    assert_eq!(
        body("state c <<choice>>").nodes[0].shape,
        NodeShape::Diamond
    );
}

#[test]
fn start_y_end_tambien_son_extremos() {
    let doc = body("state s <<start>>\nstate e <<end>>");

    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 1);
}

#[test]
fn un_estereotipo_desconocido_avisa_y_sigue() {
    let result = parse("@startuml\nstate x <<raro>>\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownStereotype"]);
    assert_eq!(result.document.nodes[0].shape, NodeShape::RoundedRect);
}

/* --- Compuestos ------------------------------------------------------------ */

#[test]
fn un_estado_compuesto_es_un_contenedor() {
    let doc = body("state Activo {\n  Nuevo --> Pagado\n}");

    let compuesto = node(&doc, "Activo");
    assert_eq!(compuesto.shape, NodeShape::Package);
    assert_eq!(doc.children_of(Some(&compuesto.id)).count(), 2);
}

#[test]
fn los_compuestos_se_anidan() {
    let doc = body("state Fuera {\n  state Dentro {\n    A --> B\n  }\n}");

    let fuera = node(&doc, "Fuera");
    let dentro = node(&doc, "Dentro");

    assert_eq!(dentro.parent.as_deref(), Some(fuera.id.as_str()));
    assert_eq!(doc.children_of(Some(&dentro.id)).count(), 2);
}

#[test]
fn una_region_concurrente_se_avisa_y_lo_de_dentro_se_queda_en_el_estado() {
    let result = parse(
        "@startuml\n\
         state Activo {\n\
         \x20 A --> B\n\
         \x20 --\n\
         \x20 C --> D\n\
         }\n\
         @enduml",
    );

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredRegion"]);

    let doc = result.document;
    let compuesto = node(&doc, "Activo");
    assert_eq!(doc.children_of(Some(&compuesto.id)).count(), 4);
}

#[test]
fn un_compuesto_vacio_se_descarta() {
    assert_eq!(body("state Vacio {\n}\nstate A").nodes.len(), 1);
}

#[test]
fn una_llave_de_mas_es_un_error() {
    let result = parse("@startuml\nstate A\n}\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

#[test]
fn un_compuesto_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nstate A {\n  B --> C\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedState"]);
}

/* --- Transiciones ---------------------------------------------------------- */

#[test]
fn una_transicion_es_una_flecha_dirigida() {
    let doc = body("A --> B");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
}

#[test]
fn lee_la_etiqueta_de_la_transicion() {
    assert_eq!(
        body("Nuevo --> Pagado : confirmar").edges[0]
            .label
            .as_deref(),
        Some("confirmar")
    );
}

#[test]
fn las_pistas_de_direccion_no_parten_la_flecha() {
    let doc = body("A -down-> B\nC -right-> D");

    assert_eq!(doc.edges.len(), 2);
    assert_eq!(doc.nodes.len(), 4);
}

#[test]
fn una_flecha_al_reves_invierte_la_transicion() {
    let doc = body("state A\nstate B\nB <-- A");

    assert_eq!(doc.edges[0].source.node, doc.nodes[0].id, "manda A");
    assert_eq!(doc.edges[0].target.node, doc.nodes[1].id, "recibe B");
}

#[test]
fn convierte_los_saltos_de_linea() {
    assert_eq!(
        body("A --> B : primera\\nsegunda").edges[0]
            .label
            .as_deref(),
        Some("primera\nsegunda")
    );
}

#[test]
fn una_transicion_sin_estado_a_un_lado_es_un_error() {
    let result = parse("@startuml\n--> B\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

/* --- Directivas ------------------------------------------------------------ */

#[test]
fn hide_y_skinparam_se_avisan_y_se_siguen() {
    let result = parse("@startuml\nhide empty description\nskinparam x y\nstate A\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective", "ignoredDirective"]);
    assert_eq!(result.document.nodes.len(), 1);
}

#[test]
fn una_nota_de_varias_lineas_se_salta_entera() {
    let result = parse(
        "@startuml\n\
         state A\n\
         note right of A\n\
         \x20 texto\n\
         \x20 B --> C\n\
         end note\n\
         state D\n\
         @enduml",
    );

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 2, "lo de dentro no cuenta");
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
@startuml
title Ciclo del pedido
hide empty description

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
"#);

    assert_eq!(doc.meta.title, "Ciclo del pedido");
    assert_eq!(count(&doc, NodeShape::Bar), 2);
    assert_eq!(count(&doc, NodeShape::Package), 1);
    assert_eq!(count(&doc, NodeShape::InitialNode), 2, "uno por nivel");
    assert_eq!(count(&doc, NodeShape::FinalNode), 2);

    assert_eq!(
        node(&doc, "Nuevo\nesperando pago").shape,
        NodeShape::RoundedRect
    );

    let pagado = node(&doc, "Pagado");
    assert_eq!(doc.children_of(Some(&pagado.id)).count(), 4);
}

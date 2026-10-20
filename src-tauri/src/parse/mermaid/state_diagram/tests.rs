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

/// Importa un cuerpo, poniendole la cabecera.
fn body(source: &str) -> Document {
    ok(&format!("stateDiagram-v2\n{source}"))
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

/// Cuantos nodos hay de esa forma.
fn count(doc: &Document, shape: NodeShape) -> usize {
    doc.nodes.iter().filter(|n| n.shape == shape).count()
}

/* --- Estados -------------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_estados_y_llega_sin_geometria() {
    let doc = body("  [*] --> Nuevo");

    assert_eq!(doc.kind, DiagramKind::State);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn un_estado_se_dibuja_redondeado() {
    assert_eq!(
        node(&body("  state Nuevo"), "Nuevo").shape,
        NodeShape::RoundedRect
    );
}

#[test]
fn mencionar_un_estado_en_una_transicion_lo_crea() {
    let doc = body("  Nuevo --> Pagado");

    assert_eq!(count(&doc, NodeShape::RoundedRect), 2);
}

#[test]
fn una_descripcion_entre_comillas_cambia_lo_que_se_dibuja() {
    let doc = body("  state \"Pendiente de pago\" as p\n  p --> Fin");

    assert_eq!(
        node(&doc, "Pendiente de pago").shape,
        NodeShape::RoundedRect
    );
}

#[test]
fn los_dos_puntos_tambien_le_ponen_texto_a_un_estado() {
    let doc = body("  state p\n  p : Pendiente de pago");

    assert_eq!(doc.nodes[0].label, "Pendiente de pago");
}

#[test]
fn lee_la_direccion() {
    assert_eq!(
        body("  direction LR\n  state A").direction,
        LayoutDirection::Right
    );
    assert_eq!(
        body("  direction TB\n  state A").direction,
        LayoutDirection::Down
    );
}

#[test]
fn se_salta_los_comentarios() {
    assert_eq!(body("  %% nada\n  state A").nodes.len(), 1);
}

/* --- Extremos -------------------------------------------------------------- */

#[test]
fn el_asterisco_a_la_izquierda_es_el_disco_de_arranque() {
    let doc = body("  [*] --> Nuevo");

    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 0);
}

#[test]
fn y_a_la_derecha_el_anillo_de_salida() {
    let doc = body("  Nuevo --> [*]");

    assert_eq!(count(&doc, NodeShape::InitialNode), 0);
    assert_eq!(count(&doc, NodeShape::FinalNode), 1);
}

#[test]
fn hay_un_extremo_de_cada_por_nivel_no_uno_por_linea() {
    // Dos lineas que empiecen en `[*]` salen del mismo disco, que es como lo
    // dibuja Mermaid.
    let doc = body("  [*] --> A\n  [*] --> B\n  A --> [*]\n  B --> [*]");

    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 1);
    assert_eq!(doc.edges.len(), 4);
}

#[test]
fn cada_nivel_tiene_los_suyos() {
    let doc = body(
        "  [*] --> Fuera\n\
         \x20 state Fuera {\n\
         \x20   [*] --> Dentro\n\
         \x20   Dentro --> [*]\n\
         \x20 }\n\
         \x20 Fuera --> [*]",
    );

    // Uno de cada en la raiz y uno de cada dentro del compuesto.
    assert_eq!(count(&doc, NodeShape::InitialNode), 2);
    assert_eq!(count(&doc, NodeShape::FinalNode), 2);

    let compuesto = node(&doc, "Fuera");
    assert_eq!(doc.children_of(Some(&compuesto.id)).count(), 3);
}

/* --- Estereotipos ---------------------------------------------------------- */

#[test]
fn fork_y_join_se_dibujan_como_barras() {
    let doc = body("  state f <<fork>>\n  state j <<join>>");

    assert_eq!(count(&doc, NodeShape::Bar), 2);
}

#[test]
fn choice_se_dibuja_como_rombo() {
    let doc = body("  state c <<choice>>");

    assert_eq!(doc.nodes[0].shape, NodeShape::Diamond);
}

#[test]
fn un_estereotipo_desconocido_avisa_y_sigue() {
    let result = parse("stateDiagram-v2\n  state x <<raro>>");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownStereotype"]);
    assert_eq!(result.document.nodes[0].shape, NodeShape::RoundedRect);
}

#[test]
fn declarar_el_estereotipo_despues_de_usar_el_estado_le_cambia_la_forma() {
    let doc = body("  A --> f\n  state f <<fork>>");

    assert_eq!(node(&doc, "f").shape, NodeShape::Bar);
}

/* --- Compuestos ------------------------------------------------------------ */

#[test]
fn un_estado_compuesto_es_un_contenedor() {
    let doc = body("  state Activo {\n    Nuevo --> Pagado\n  }");

    let compuesto = node(&doc, "Activo");
    assert_eq!(compuesto.shape, NodeShape::Package);
    assert_eq!(doc.children_of(Some(&compuesto.id)).count(), 2);
}

#[test]
fn los_compuestos_se_anidan() {
    let doc = body(
        "  state Fuera {\n\
         \x20   state Dentro {\n\
         \x20     A --> B\n\
         \x20   }\n\
         \x20 }",
    );

    let fuera = node(&doc, "Fuera");
    let dentro = node(&doc, "Dentro");

    assert_eq!(dentro.parent.as_deref(), Some(fuera.id.as_str()));
    assert_eq!(doc.children_of(Some(&dentro.id)).count(), 2);
}

#[test]
fn un_compuesto_con_descripcion_tambien_vale() {
    let doc = body("  state \"En curso\" as c {\n    A --> B\n  }");

    let compuesto = node(&doc, "En curso");
    assert_eq!(doc.children_of(Some(&compuesto.id)).count(), 2);
}

#[test]
fn un_compuesto_vacio_se_descarta() {
    let doc = body("  state Vacio {\n  }\n  state A");

    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn una_llave_de_mas_es_un_error() {
    let result = parse("stateDiagram-v2\n  state A\n  }");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

#[test]
fn un_compuesto_sin_cerrar_es_un_error() {
    let result = parse("stateDiagram-v2\n  state A {\n    B --> C");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedState"]);
}

/* --- Transiciones ---------------------------------------------------------- */

#[test]
fn una_transicion_es_una_flecha_dirigida() {
    let doc = body("  A --> B");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
}

#[test]
fn lee_la_etiqueta_de_la_transicion() {
    let doc = body("  Nuevo --> Pagado : confirmar");

    assert_eq!(doc.edges[0].label.as_deref(), Some("confirmar"));
}

#[test]
fn convierte_los_saltos_de_linea_de_html() {
    let doc = body("  A --> B : primera<br/>segunda");

    assert_eq!(doc.edges[0].label.as_deref(), Some("primera\nsegunda"));
}

#[test]
fn una_transicion_sin_estado_a_un_lado_es_un_error() {
    let result = parse("stateDiagram-v2\n  --> B");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

/* --- Lo que no se guarda ---------------------------------------------------- */

#[test]
fn una_nota_de_una_linea_se_avisa_y_se_salta() {
    let result = parse("stateDiagram-v2\n  state A\n  note right of A : ojo\n  state B");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredNote"]);
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn una_nota_de_varias_lineas_se_salta_entera() {
    let result = parse(
        "stateDiagram-v2\n\
         \x20 state A\n\
         \x20 note right of A\n\
         \x20   texto largo\n\
         \x20   B --> C\n\
         \x20 end note\n\
         \x20 state D",
    );

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 2, "lo de dentro no cuenta");
}

#[test]
fn classdef_se_avisa_y_se_sigue() {
    let result = parse("stateDiagram-v2\n  classDef x fill:#f9f\n  state A");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective"]);
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok("stateDiagram-v2\n\
         \x20 direction LR\n\
         \n\
         \x20 [*] --> Nuevo\n\
         \x20 Nuevo --> Pagado : confirmar\n\
         \n\
         \x20 state Pagado {\n\
         \x20   [*] --> Cobrando\n\
         \x20   Cobrando --> Cobrado\n\
         \x20   Cobrado --> [*]\n\
         \x20 }\n\
         \n\
         \x20 state reparto <<fork>>\n\
         \x20 Pagado --> reparto\n\
         \x20 reparto --> Enviado\n\
         \x20 reparto --> Facturado\n\
         \n\
         \x20 state cierre <<join>>\n\
         \x20 Enviado --> cierre\n\
         \x20 Facturado --> cierre\n\
         \x20 cierre --> [*]");

    assert_eq!(doc.direction, LayoutDirection::Right);
    assert_eq!(count(&doc, NodeShape::Bar), 2);
    assert_eq!(count(&doc, NodeShape::Package), 1, "`Pagado` es compuesto");

    // Un extremo de cada por nivel: raiz y dentro de `Pagado`.
    assert_eq!(count(&doc, NodeShape::InitialNode), 2);
    assert_eq!(count(&doc, NodeShape::FinalNode), 2);

    let pagado = node(&doc, "Pagado");
    assert_eq!(doc.children_of(Some(&pagado.id)).count(), 4);

    assert_eq!(doc.edges[1].label.as_deref(), Some("confirmar"));
}

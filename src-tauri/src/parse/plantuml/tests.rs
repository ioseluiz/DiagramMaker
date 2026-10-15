use super::*;
use crate::model::EdgeKind;

fn parse(source: &str) -> ImportResult {
    import(source, 0)
}

/// Importa esperando que no haya ningun error.
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
        .unwrap_or_else(|| panic!("no hay ningun elemento `{label}`"))
}

/* --- Armazon y directivas ------------------------------------------------ */

#[test]
fn el_diagrama_importado_es_de_arquitectura() {
    assert_eq!(body("[A]").kind, DiagramKind::Architecture);
}

#[test]
fn el_documento_importado_llega_sin_geometria() {
    let doc = body("[A] --> [B]");

    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn sin_startuml_avisa_pero_importa() {
    let result = parse("[A] --> [B]");

    assert!(result.ok());
    assert_eq!(codes(&result), ["missingHeader"]);
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn un_startuml_con_nombre_tambien_vale() {
    let doc = ok("@startuml plataforma\n[A]\n@enduml");

    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn lee_el_titulo() {
    assert_eq!(body("title Mi plataforma\n[A]").meta.title, "Mi plataforma");
}

#[test]
fn lee_las_dos_direcciones() {
    assert_eq!(
        body("left to right direction\n[A]").direction,
        LayoutDirection::Right
    );
    assert_eq!(
        body("top to bottom direction\n[A]").direction,
        LayoutDirection::Down
    );
}

#[test]
fn se_salta_los_comentarios() {
    let doc = body("' esto es un comentario\n[A]\n/' y esto\n   tambien '/\n[B]");

    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn un_comentario_de_bloque_en_una_linea_no_se_come_el_resto() {
    let doc = body("/' nota '/\n[A]\n[B]");

    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn avisa_de_las_directivas_que_no_tienen_efecto() {
    let result = parse("@startuml\nskinparam monochrome true\n[A]\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective"]);
    assert_eq!(result.document.nodes.len(), 1);
}

#[test]
fn avisa_de_un_include_porque_lo_que_traiga_no_entra() {
    let result = parse("@startuml\n!include <C4/C4_Container>\n[A]\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredPreprocessor"]);
}

#[test]
fn un_skinparam_con_bloque_no_se_lleva_los_elementos_por_delante() {
    let result = parse("@startuml\nskinparam component {\n  BorderColor red\n}\n[A]\n@enduml");

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 1);
    assert!(result.document.nodes[0].parent.is_none());
}

/* --- Elementos ----------------------------------------------------------- */

#[test]
fn reconoce_las_palabras_clave_de_cada_forma() {
    let doc = body(
        "component Caja\n\
         rectangle Rect\n\
         database Base\n\
         storage Alma\n\
         queue Cola\n\
         interface Puerto\n\
         usecase Caso\n\
         actor Persona\n\
         card Tarjeta\n\
         cloud Nube",
    );

    assert_eq!(node(&doc, "Caja").shape, NodeShape::Rectangle);
    assert_eq!(node(&doc, "Rect").shape, NodeShape::Rectangle);
    assert_eq!(node(&doc, "Base").shape, NodeShape::Cylinder);
    assert_eq!(node(&doc, "Alma").shape, NodeShape::Cylinder);
    assert_eq!(node(&doc, "Cola").shape, NodeShape::Hexagon);
    assert_eq!(node(&doc, "Puerto").shape, NodeShape::Ellipse);
    assert_eq!(node(&doc, "Caso").shape, NodeShape::Ellipse);
    assert_eq!(node(&doc, "Persona").shape, NodeShape::Actor);
    assert_eq!(node(&doc, "Tarjeta").shape, NodeShape::RoundedRect);
    assert_eq!(node(&doc, "Nube").shape, NodeShape::RoundedRect);
}

#[test]
fn los_corchetes_son_la_forma_corta_de_component() {
    let doc = body("[Portal web]");

    assert_eq!(doc.nodes[0].label, "Portal web");
    assert_eq!(doc.nodes[0].shape, NodeShape::Rectangle);
}

#[test]
fn un_alias_sirve_para_referenciar() {
    let doc = body("component \"API REST\" as api\n[Portal] --> api");

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(node(&doc, "API REST").shape, NodeShape::Rectangle);
}

#[test]
fn el_nombre_visible_tambien_sirve_para_referenciar() {
    // PlantUML acepta las dos formas, y mezclarlas es corriente.
    let doc = body("component \"API REST\" as api\n[Portal] --> \"API REST\"");

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.edges.len(), 1);
}

#[test]
fn declarar_la_forma_despues_de_usar_el_elemento_la_aplica() {
    let doc = body("api --> base\ndatabase base");

    assert_eq!(node(&doc, "base").shape, NodeShape::Cylinder);
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn lee_un_color_con_nombre() {
    let doc = body("component Caja #LightBlue");

    assert_eq!(doc.nodes[0].style.fill.as_deref(), Some("LightBlue"));
}

#[test]
fn a_un_color_hexadecimal_le_devuelve_la_almohadilla() {
    let doc = body("component Caja #AACCFF");

    assert_eq!(doc.nodes[0].style.fill.as_deref(), Some("#AACCFF"));
}

#[test]
fn un_estereotipo_no_estorba() {
    let doc = body("component \"Pagos\" as p <<servicio>>");

    assert_eq!(doc.nodes[0].label, "Pagos");
    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn dos_nombres_que_se_limpian_igual_no_colisionan() {
    // `api-rest` y `api.rest` dan el mismo nombre saneado; los ids tienen
    // que seguir siendo distintos.
    let doc = body("[api-rest]\n[api.rest]");

    assert_eq!(doc.nodes.len(), 2);
    assert_ne!(doc.nodes[0].id, doc.nodes[1].id);
}

/* --- Relaciones ---------------------------------------------------------- */

#[test]
fn reconoce_los_tipos_de_flecha() {
    let doc = body("a --> b\nc -- d\ne ..> f\ng .. h\ni --|> j\nk ..|> l\nm o-- n\no *-- p");

    let kinds: Vec<_> = doc.edges.iter().map(|e| e.kind).collect();
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
fn una_flecha_al_reves_invierte_la_relacion() {
    let doc = body("[A] <-- [B]");

    assert_eq!(doc.edges[0].source.node, node(&doc, "B").id);
    assert_eq!(doc.edges[0].target.node, node(&doc, "A").id);
    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
}

#[test]
fn el_rombo_se_queda_en_el_origen_este_donde_este() {
    // `A o-- B` y `B --o A` dicen lo mismo: el todo es A.
    let directo = body("[A] o-- [B]");
    let inverso = body("[B] --o [A]");

    assert_eq!(directo.edges[0].source.node, node(&directo, "A").id);
    assert_eq!(inverso.edges[0].source.node, node(&inverso, "A").id);
}

#[test]
fn lee_la_etiqueta_detras_de_los_dos_puntos() {
    let doc = body("[A] --> [B] : consulta");

    assert_eq!(doc.edges[0].label.as_deref(), Some("consulta"));
}

#[test]
fn una_etiqueta_puede_llevar_dos_puntos_dentro() {
    let doc = body("[A] --> [B] : usa el puerto 443: TLS");

    assert_eq!(
        doc.edges[0].label.as_deref(),
        Some("usa el puerto 443: TLS")
    );
}

#[test]
fn convierte_los_saltos_de_linea_de_una_etiqueta() {
    let doc = body("[A] --> [B] : primera\\nsegunda");

    assert_eq!(doc.edges[0].label.as_deref(), Some("primera\nsegunda"));
}

#[test]
fn lee_las_cardinalidades_de_los_extremos() {
    let doc = body("api \"1\" --> \"0..*\" base");

    assert_eq!(doc.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(doc.edges[0].target.label.as_deref(), Some("0..*"));
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn un_nombre_entrecomillado_no_es_una_cardinalidad() {
    let doc = body("\"API REST\" --> base");

    assert_eq!(doc.edges[0].source.label, None);
    assert_eq!(node(&doc, "API REST").shape, NodeShape::Rectangle);
}

#[test]
fn las_pistas_de_direccion_no_parten_la_flecha() {
    let doc = body("[A] -down-> [B]\n[C] -r-> [D]");

    assert_eq!(doc.edges.len(), 2);
    assert_eq!(doc.nodes.len(), 4);
    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
}

#[test]
fn el_color_de_una_flecha_no_parte_la_linea() {
    let doc = body("[A] -[#red]-> [B]");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn un_estilo_punteado_dentro_de_la_flecha_se_respeta() {
    let doc = body("[A] -[dashed]-> [B]");

    assert_eq!(doc.edges[0].style.dashed, Some(true));
}

#[test]
fn un_guion_dentro_de_un_nombre_no_es_una_flecha() {
    // Sin la regla de los limites, `api-rest` seria una relacion.
    let doc = body("[api-rest] --> [base]");

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(node(&doc, "api-rest").label, "api-rest");
}

#[test]
fn una_o_al_final_de_un_nombre_no_es_una_agregacion() {
    let doc = body("banco--parte");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.edges[0].kind, EdgeKind::Association);
    assert_eq!(node(&doc, "banco").label, "banco");
}

#[test]
fn una_flecha_dentro_de_una_etiqueta_no_parte_la_linea() {
    let doc = body("component \"entrada --> salida\" as a");

    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].label, "entrada --> salida");
}

#[test]
fn usar_un_componente_sin_declararlo_lo_crea() {
    let doc = body("[A] --> [B]");

    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.nodes.iter().all(|n| n.shape == NodeShape::Rectangle));
}

/* --- Bloques ------------------------------------------------------------- */

#[test]
fn un_package_se_convierte_en_contenedor() {
    let doc = body("package \"Servicios\" {\n  [api]\n  [cola]\n}\n[web] --> [api]");

    let group = node(&doc, "Servicios");
    assert_eq!(group.shape, NodeShape::Package);
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn los_bloques_se_anidan() {
    let doc = body("package Fuera {\n  package Dentro {\n    [Hoja]\n  }\n}");

    let fuera = node(&doc, "Fuera");
    let dentro = node(&doc, "Dentro");

    assert_eq!(dentro.parent.as_deref(), Some(fuera.id.as_str()));
    assert_eq!(
        node(&doc, "Hoja").parent.as_deref(),
        Some(dentro.id.as_str())
    );
}

#[test]
fn cualquier_palabra_que_abra_bloque_dibuja_un_contenedor() {
    // `node`, `cloud` o `folder` con llaves contienen cosas; sin un
    // contenedor visible no se veria que estan dentro.
    let doc = body("node \"Infra\" {\n  [api]\n}");

    assert_eq!(node(&doc, "Infra").shape, NodeShape::Package);
}

#[test]
fn un_package_vacio_se_descarta() {
    let doc = body("package Vacio {\n}\n[A]");

    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].label, "A");
}

#[test]
fn together_agrupa_sin_dibujar_caja() {
    let doc = body("together {\n  [A]\n  [B]\n}");

    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.nodes.iter().all(|n| n.parent.is_none()));
}

#[test]
fn un_together_dentro_de_un_package_deja_los_hijos_en_el_package() {
    let doc = body("package Capa {\n  together {\n    [A]\n  }\n}");

    let capa = node(&doc, "Capa");
    assert_eq!(node(&doc, "A").parent.as_deref(), Some(capa.id.as_str()));
}

#[test]
fn una_llave_sin_abrir_es_un_error() {
    let result = parse("@startuml\n[A]\n}\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

#[test]
fn un_bloque_sin_cerrar_es_un_error() {
    let result = parse("@startuml\npackage Capa {\n  [A]\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedBlock"]);
}

/* --- Notas --------------------------------------------------------------- */

#[test]
fn una_nota_de_una_linea_se_avisa_y_se_salta() {
    let result = parse("@startuml\n[A]\nnote left of A : cuidado\n[B]\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredNote"]);
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn una_nota_de_varias_lineas_se_salta_entera() {
    let result = parse(
        "@startuml\n\
         [A]\n\
         note right of A\n\
           esto no entra\n\
           [esto tampoco]\n\
         end note\n\
         [B]\n\
         @enduml",
    );

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn una_leyenda_se_salta_entera() {
    let result = parse("@startuml\n[A]\nlegend\n  texto libre\nend legend\n[B]\n@enduml");

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 2);
}

/* --- Errores ------------------------------------------------------------- */

#[test]
fn una_linea_que_no_se_entiende_se_reporta_con_su_posicion() {
    let result = parse("@startuml\n[A]\n  ??? raro\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badStatement"]);

    let span = result.diagnostics[0].span.as_ref().unwrap();
    assert_eq!((span.line, span.column), (3, 3));
}

#[test]
fn un_error_no_se_lleva_por_delante_el_resto_del_diagrama() {
    let result = parse("@startuml\n??? raro\n[A] --> [B]\n@enduml");

    assert_eq!(result.document.nodes.len(), 2);
    assert_eq!(result.document.edges.len(), 1);
}

#[test]
fn una_flecha_sin_elemento_a_un_lado_es_un_error() {
    let result = parse("@startuml\n--> [B]\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

/* --- Diagrama completo --------------------------------------------------- */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
@startuml plataforma
title Plataforma de pagos
left to right direction

skinparam componentStyle rectangle

actor "Cliente" as cliente

package "Capa publica" {
  [Portal web] as web
  component "API REST" as api
}

package "Datos" {
  database "PostgreSQL" as base #AACCFF
  queue "Eventos" as cola
}

cloud "Pasarela externa" as pasarela

cliente --> web : HTTPS
web --> api
api "1" --> "0..*" base
api ..> cola : publica
api -[#red]-> pasarela : cobro
@enduml
"#);

    assert_eq!(doc.meta.title, "Plataforma de pagos");
    assert_eq!(doc.direction, LayoutDirection::Right);
    assert_eq!(doc.nodes.len(), 8);
    assert_eq!(doc.edges.len(), 5);

    assert_eq!(node(&doc, "Cliente").shape, NodeShape::Actor);
    assert_eq!(node(&doc, "PostgreSQL").shape, NodeShape::Cylinder);
    assert_eq!(node(&doc, "Eventos").shape, NodeShape::Hexagon);
    assert_eq!(node(&doc, "Pasarela externa").shape, NodeShape::RoundedRect);

    let datos = node(&doc, "Datos");
    assert_eq!(doc.children_of(Some(&datos.id)).count(), 2);

    assert_eq!(doc.edges[2].source.label.as_deref(), Some("1"));
    assert_eq!(doc.edges[3].kind, EdgeKind::Dependency);
}

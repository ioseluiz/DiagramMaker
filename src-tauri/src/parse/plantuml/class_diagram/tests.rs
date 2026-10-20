use super::*;
use crate::model::EdgeKind;

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

/// Caja de la clase con esa etiqueta.
fn class<'a>(doc: &'a Document, label: &str) -> &'a ClassBox {
    doc.nodes
        .iter()
        .find(|n| n.label == label)
        .and_then(|n| n.data.as_class())
        .unwrap_or_else(|| panic!("no hay ninguna clase `{label}`"))
}

/* --- Clases --------------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_clases_y_llega_sin_geometria() {
    let doc = body("class Pedido");

    assert_eq!(doc.kind, DiagramKind::Class);
    assert_eq!(doc.nodes[0].shape, NodeShape::Class);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn cada_palabra_implica_su_estereotipo() {
    // Lo que en Mermaid es `<<interface>>` aqui es una palabra propia.
    let doc = body(
        "class Pedido\n\
         interface Repositorio\n\
         abstract class Base\n\
         enum Estado\n\
         annotation Marca",
    );

    assert_eq!(class(&doc, "Pedido").stereotype, None);
    assert_eq!(
        class(&doc, "Repositorio").stereotype.as_deref(),
        Some("interface")
    );
    assert_eq!(class(&doc, "Base").stereotype.as_deref(), Some("abstract"));
    assert_eq!(
        class(&doc, "Estado").stereotype.as_deref(),
        Some("enumeration")
    );
    assert_eq!(
        class(&doc, "Marca").stereotype.as_deref(),
        Some("annotation")
    );
}

#[test]
fn un_estereotipo_escrito_a_mano_gana_a_la_palabra() {
    let doc = body("class Pedido <<servicio>>");

    assert_eq!(
        class(&doc, "Pedido").stereotype.as_deref(),
        Some("servicio")
    );
}

#[test]
fn un_nombre_entrecomillado_con_alias_sirve_por_los_dos() {
    let doc = body(
        "class \"Pedido de compra\" as pedido\n\
         pedido --> Linea\n\
         \"Pedido de compra\" --> Otra",
    );

    assert_eq!(doc.nodes[0].label, "Pedido de compra");
    assert_eq!(doc.edges.len(), 2);
    assert_eq!(doc.edges[0].source.node, doc.edges[1].source.node);
}

#[test]
fn mencionar_una_clase_en_una_relacion_la_crea() {
    let doc = body("Pedido <|-- PedidoEspecial");

    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.nodes.iter().all(|n| n.shape == NodeShape::Class));
}

#[test]
fn lee_el_titulo() {
    assert_eq!(body("title Dominio\nclass A").meta.title, "Dominio");
}

#[test]
fn se_salta_los_comentarios() {
    let doc = body("' esto no cuenta\nclass A\n/' ni\nesto '/\nclass B");

    assert_eq!(doc.nodes.len(), 2);
}

/* --- Miembros ------------------------------------------------------------- */

#[test]
fn un_atributo_lleva_el_tipo_detras_de_los_dos_puntos() {
    // Al reves que en Mermaid: `- id : int`, no `-int id`.
    let doc = body("class Pedido {\n  - id : int\n}");
    let member = &class(&doc, "Pedido").attributes[0];

    assert_eq!(member.name, "id");
    assert_eq!(member.type_name, "int");
    assert_eq!(member.visibility, Visibility::Private);
}

#[test]
fn tambien_admite_la_forma_de_c() {
    let doc = body("class Pedido {\n  -int id\n}");
    let member = &class(&doc, "Pedido").attributes[0];

    assert_eq!(member.name, "id");
    assert_eq!(member.type_name, "int");
}

#[test]
fn un_atributo_sin_tipo_vale() {
    let doc = body("class Pedido {\n  + activo\n}");

    assert_eq!(class(&doc, "Pedido").attributes[0].type_name, "");
}

#[test]
fn los_parentesis_hacen_que_sea_una_operacion() {
    let doc = body("class Pedido {\n  + confirmar()\n}");
    let box_ = class(&doc, "Pedido");

    assert!(box_.attributes.is_empty());
    assert_eq!(box_.operations.len(), 1);
}

#[test]
fn una_operacion_lee_sus_parametros_y_lo_que_devuelve() {
    let doc = body("class Pedido {\n  + buscar(id : int, activo : bool) : Pedido\n}");
    let operation = &class(&doc, "Pedido").operations[0];

    assert_eq!(operation.parameters, "id : int, activo : bool");
    assert_eq!(operation.type_name, "Pedido");
}

#[test]
fn reconoce_las_cuatro_visibilidades() {
    let doc = body(
        "class Pedido {\n\
         \x20 + publico\n\
         \x20 - privado\n\
         \x20 # protegido\n\
         \x20 ~ paquete\n\
         }",
    );

    let visibilities: Vec<_> = class(&doc, "Pedido")
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
fn las_llaves_marcan_lo_estatico_y_lo_abstracto() {
    let doc = body("class Pedido {\n  + {static} creados : int\n  + {abstract} calcular()\n}");
    let box_ = class(&doc, "Pedido");

    assert!(box_.attributes[0].is_static);
    assert_eq!(box_.attributes[0].name, "creados");
    assert!(box_.operations[0].is_abstract);
}

#[test]
fn field_y_method_mandan_sobre_los_parentesis() {
    // Sin ellos, los parentesis deciden; con ellos, no hay duda.
    let doc = body("class Pedido {\n  {field} raro()\n  {method} tambien\n}");
    let box_ = class(&doc, "Pedido");

    assert_eq!(box_.attributes.len(), 1, "`{{field}}` manda");
    assert_eq!(box_.operations.len(), 1, "`{{method}}` manda");
}

#[test]
fn un_separador_de_compartimento_no_es_un_miembro() {
    let doc = body("class Pedido {\n  - id : int\n  --\n  + confirmar()\n}");
    let box_ = class(&doc, "Pedido");

    assert_eq!(box_.attributes.len(), 1);
    assert_eq!(box_.operations.len(), 1);
}

#[test]
fn los_valores_de_un_enum_entran_como_atributos() {
    let doc = body("enum Estado {\n  NUEVO\n  PAGADO\n}");
    let box_ = class(&doc, "Estado");

    assert_eq!(box_.attributes.len(), 2);
    assert_eq!(box_.attributes[0].name, "NUEVO");
}

#[test]
fn un_estereotipo_dentro_del_bloque_tambien_se_lee() {
    let doc = body("class Repositorio {\n  <<interface>>\n  + guardar()\n}");
    let box_ = class(&doc, "Repositorio");

    assert_eq!(box_.stereotype.as_deref(), Some("interface"));
    assert_eq!(box_.operations.len(), 1, "el estereotipo no es un miembro");
}

/* --- Relaciones ----------------------------------------------------------- */

#[test]
fn reconoce_los_tipos_de_relacion() {
    let doc = body(
        "A <|-- B\n\
         C *-- D\n\
         E o-- F\n\
         G --> H\n\
         I ..> J\n\
         K ..|> L\n\
         M -- N",
    );

    let kinds: Vec<_> = doc.edges.iter().map(|e| e.kind).collect();
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
fn la_herencia_apunta_al_padre_lo_escriba_como_lo_escriba() {
    let izquierda = body("class A\nclass B\nA <|-- B");
    let derecha = body("class B\nclass A\nB --|> A");

    let padre = |doc: &Document| doc.node(&doc.edges[0].target.node).unwrap().label.clone();

    assert_eq!(padre(&izquierda), "A");
    assert_eq!(padre(&derecha), "A");
}

#[test]
fn lee_la_etiqueta_y_las_cardinalidades() {
    let doc = body("Pedido \"1\" *-- \"0..*\" Linea : contiene");

    assert_eq!(doc.edges[0].label.as_deref(), Some("contiene"));
    assert_eq!(doc.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(doc.edges[0].target.label.as_deref(), Some("0..*"));
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn una_relacion_sin_clase_a_un_lado_es_un_error() {
    let result = parse("@startuml\n--> B\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

/* --- Paquetes -------------------------------------------------------------- */

#[test]
fn un_package_se_convierte_en_contenedor() {
    let doc = body("package Dominio {\n  class Pedido\n  class Linea\n}");

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el contenedor");

    assert_eq!(group.label, "Dominio");
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn un_namespace_hace_lo_mismo() {
    let doc = body("namespace Dominio {\n  class Pedido\n}");

    assert!(doc.nodes.iter().any(|n| n.shape == NodeShape::Package));
}

#[test]
fn el_cierre_de_una_clase_no_cierra_su_paquete() {
    // Las dos cosas se cierran con `}`; sin distinguirlas, la clase se
    // llevaria por delante el contenedor.
    let doc = body(
        "package Dominio {\n\
         \x20 class Pedido {\n\
         \x20   - id : int\n\
         \x20 }\n\
         \x20 class Linea\n\
         }",
    );

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .unwrap();
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
    assert_eq!(class(&doc, "Pedido").attributes.len(), 1);
}

#[test]
fn un_paquete_vacio_se_descarta() {
    let doc = body("package Vacio {\n}\nclass A");

    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn una_llave_de_mas_es_un_error() {
    let result = parse("@startuml\nclass A\n}\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

#[test]
fn un_bloque_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nclass A {\n  - id : int\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedClass"]);
}

/* --- Directivas ------------------------------------------------------------ */

#[test]
fn hide_y_skinparam_se_avisan_y_se_siguen() {
    let result =
        parse("@startuml\nhide empty members\nskinparam monochrome true\nclass A\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective", "ignoredDirective"]);
    assert_eq!(result.document.nodes.len(), 1);
}

#[test]
fn una_nota_de_varias_lineas_se_salta_entera() {
    let result = parse(
        "@startuml\n\
         class A\n\
         note top of A\n\
         \x20 texto largo\n\
         end note\n\
         class B\n\
         @enduml",
    );

    assert!(result.ok(), "{:#?}", result.diagnostics);
    assert_eq!(result.document.nodes.len(), 2);
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
@startuml
title Dominio de pedidos
hide empty members

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
"#);

    assert_eq!(doc.meta.title, "Dominio de pedidos");
    assert_eq!(doc.edges.len(), 4);

    let pedido = class(&doc, "Pedido");
    assert_eq!(pedido.attributes.len(), 3);
    assert_eq!(pedido.operations.len(), 2);
    assert!(pedido.attributes[2].is_static);
    assert_eq!(pedido.operations[1].type_name, "Pedido");

    let repo = class(&doc, "Repositorio");
    assert_eq!(repo.stereotype.as_deref(), Some("interface"));
    assert!(repo.operations[0].is_abstract);

    assert_eq!(class(&doc, "Base").stereotype.as_deref(), Some("abstract"));

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el paquete");
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

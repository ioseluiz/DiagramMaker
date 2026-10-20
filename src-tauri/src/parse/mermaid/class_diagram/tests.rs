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
    ok(&format!("classDiagram\n{source}"))
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
    let doc = body("  class Pedido");

    assert_eq!(doc.kind, DiagramKind::Class);
    assert_eq!(doc.nodes[0].shape, NodeShape::Class);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn una_clase_sin_bloque_llega_vacia() {
    let box_ = class(&body("  class Pedido"), "Pedido").clone();

    assert!(box_.attributes.is_empty());
    assert!(box_.operations.is_empty());
}

#[test]
fn una_etiqueta_entre_corchetes_cambia_lo_que_se_dibuja() {
    let doc = body("  class Pedido[\"Pedido de compra\"]");

    assert_eq!(doc.nodes[0].label, "Pedido de compra");
}

#[test]
fn mencionar_una_clase_en_una_relacion_la_crea() {
    let doc = body("  Pedido <|-- PedidoEspecial");

    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.nodes.iter().all(|n| n.shape == NodeShape::Class));
}

#[test]
fn lee_la_direccion() {
    assert_eq!(
        body("  direction LR\n  class A").direction,
        LayoutDirection::Right
    );
    assert_eq!(
        body("  direction TB\n  class A").direction,
        LayoutDirection::Down
    );
}

#[test]
fn se_salta_los_comentarios() {
    let doc = body("  %% esto no cuenta\n  class Pedido");

    assert_eq!(doc.nodes.len(), 1);
}

/* --- Miembros ------------------------------------------------------------- */

#[test]
fn un_atributo_lleva_el_tipo_delante_del_nombre() {
    // Al reves que en PlantUML: `+String nombre`, no `+nombre : String`.
    let doc = body("  class Pedido {\n    -int id\n  }");
    let box_ = class(&doc, "Pedido");

    assert_eq!(box_.attributes.len(), 1);
    assert_eq!(box_.attributes[0].name, "id");
    assert_eq!(box_.attributes[0].type_name, "int");
    assert_eq!(box_.attributes[0].visibility, Visibility::Private);
}

#[test]
fn un_atributo_sin_tipo_vale() {
    let doc = body("  class Pedido {\n    +activo\n  }");

    assert_eq!(class(&doc, "Pedido").attributes[0].type_name, "");
}

#[test]
fn los_parentesis_hacen_que_sea_una_operacion() {
    let doc = body("  class Pedido {\n    +confirmar()\n  }");
    let box_ = class(&doc, "Pedido");

    assert!(box_.attributes.is_empty());
    assert_eq!(box_.operations.len(), 1);
    assert_eq!(box_.operations[0].name, "confirmar");
}

#[test]
fn una_operacion_lee_sus_parametros_y_lo_que_devuelve() {
    let doc = body("  class Pedido {\n    +buscar(int id, bool activo) Pedido\n  }");
    let operation = &class(&doc, "Pedido").operations[0];

    assert_eq!(operation.parameters, "int id, bool activo");
    assert_eq!(operation.type_name, "Pedido");
}

#[test]
fn reconoce_las_cuatro_visibilidades() {
    let doc = body(
        "  class Pedido {\n\
         \x20   +publico\n\
         \x20   -privado\n\
         \x20   #protegido\n\
         \x20   ~paquete\n\
         \x20 }",
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
fn el_dolar_marca_lo_estatico_y_el_asterisco_lo_abstracto() {
    // El clasificador va al final de todo el miembro, detras del nombre:
    // `+int cuenta$`, no `+cuenta$ int`.
    let doc = body("  class Pedido {\n    +int cuenta$\n    +calcular()*\n  }");
    let box_ = class(&doc, "Pedido");

    assert!(box_.attributes[0].is_static);
    assert_eq!(box_.attributes[0].name, "cuenta");
    assert_eq!(box_.attributes[0].type_name, "int");
    assert!(box_.operations[0].is_abstract);
}

#[test]
fn un_miembro_se_puede_declarar_fuera_del_bloque() {
    let doc = body("  class Pedido\n  Pedido : -int id\n  Pedido : +confirmar()");
    let box_ = class(&doc, "Pedido");

    assert_eq!(box_.attributes.len(), 1);
    assert_eq!(box_.operations.len(), 1);
}

#[test]
fn un_miembro_suelto_de_una_clase_que_no_existe_es_un_error() {
    let result = parse("classDiagram\n  Fantasma : +int id");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unknownClass"]);
}

/* --- Estereotipos --------------------------------------------------------- */

#[test]
fn el_estereotipo_se_lee_dentro_del_bloque() {
    let doc = body("  class Repositorio {\n    <<interface>>\n    +guardar()\n  }");
    let box_ = class(&doc, "Repositorio");

    assert_eq!(box_.stereotype.as_deref(), Some("interface"));
    assert_eq!(box_.operations.len(), 1, "el estereotipo no es un miembro");
}

#[test]
fn el_estereotipo_tambien_se_lee_fuera() {
    let doc = body("  class Repositorio\n  <<interface>> Repositorio");

    assert_eq!(
        class(&doc, "Repositorio").stereotype.as_deref(),
        Some("interface")
    );
}

/* --- Relaciones ----------------------------------------------------------- */

#[test]
fn reconoce_los_tipos_de_relacion() {
    let doc = body(
        "  A <|-- B\n\
         \x20 C *-- D\n\
         \x20 E o-- F\n\
         \x20 G --> H\n\
         \x20 I ..> J\n\
         \x20 K ..|> L\n\
         \x20 M -- N",
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
    // `A <|-- B` y `B --|> A` dicen lo mismo: B hereda de A.
    let izquierda = body("  class A\n  class B\n  A <|-- B");
    let derecha = body("  class B\n  class A\n  B --|> A");

    let padre = |doc: &Document| {
        let edge = &doc.edges[0];
        doc.node(&edge.target.node).unwrap().label.clone()
    };

    assert_eq!(padre(&izquierda), "A");
    assert_eq!(padre(&derecha), "A");
}

#[test]
fn el_rombo_se_queda_en_el_origen_este_donde_este() {
    let directo = body("  class A\n  class B\n  A *-- B");
    let inverso = body("  class B\n  class A\n  B --* A");

    let todo = |doc: &Document| {
        let edge = &doc.edges[0];
        doc.node(&edge.source.node).unwrap().label.clone()
    };

    assert_eq!(todo(&directo), "A");
    assert_eq!(todo(&inverso), "A");
}

#[test]
fn un_enlace_punteado_sin_punta_sale_punteado() {
    let doc = body("  A .. B");

    assert_eq!(doc.edges[0].kind, EdgeKind::Association);
    assert_eq!(doc.edges[0].style.dashed, Some(true));
}

#[test]
fn lee_la_etiqueta_de_la_relacion() {
    let doc = body("  Pedido --> Linea : contiene");

    assert_eq!(doc.edges[0].label.as_deref(), Some("contiene"));
}

#[test]
fn lee_las_cardinalidades_de_los_extremos() {
    let doc = body("  Pedido \"1\" --> \"0..*\" Linea : contiene");

    assert_eq!(doc.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(doc.edges[0].target.label.as_deref(), Some("0..*"));
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn al_invertir_la_relacion_las_cardinalidades_la_siguen() {
    let doc = body("  class Hijo\n  class Padre\n  Padre \"1\" <|-- \"n\" Hijo");
    let edge = &doc.edges[0];

    // La herencia se lee al reves: el origen es el hijo.
    assert_eq!(doc.node(&edge.source.node).unwrap().label, "Hijo");
    assert_eq!(edge.source.label.as_deref(), Some("n"));
    assert_eq!(edge.target.label.as_deref(), Some("1"));
}

#[test]
fn una_relacion_sin_clase_a_un_lado_es_un_error() {
    let result = parse("classDiagram\n  --> B");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

/* --- Namespaces ----------------------------------------------------------- */

#[test]
fn un_namespace_se_convierte_en_contenedor() {
    let doc = body("  namespace Dominio {\n    class Pedido\n    class Linea\n  }");

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el contenedor");

    assert_eq!(group.label, "Dominio");
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn un_namespace_vacio_se_descarta() {
    let doc = body("  namespace Vacio {\n  }\n  class A");

    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn el_cierre_de_una_clase_no_cierra_su_namespace() {
    // Las dos cosas se cierran con `}`; sin distinguirlas, la clase se
    // llevaria por delante el contenedor.
    let doc = body(
        "  namespace Dominio {\n\
         \x20   class Pedido {\n\
         \x20     +int id\n\
         \x20   }\n\
         \x20   class Linea\n\
         \x20 }",
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
fn una_llave_de_mas_es_un_error() {
    let result = parse("classDiagram\n  class A\n  }");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

#[test]
fn un_bloque_sin_cerrar_es_un_error() {
    let result = parse("classDiagram\n  class A {\n    +int id");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedClass"]);
}

/* --- Diagrama completo ----------------------------------------------------- */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok("classDiagram\n\
         \x20 direction LR\n\
         \n\
         \x20 class Pedido {\n\
         \x20   -int id\n\
         \x20   -Decimal total\n\
         \x20   +int creados$\n\
         \x20   +confirmar()\n\
         \x20   +buscar(int id) Pedido\n\
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
         \x20 Pedido ..> Repositorio : usa");

    assert_eq!(doc.direction, LayoutDirection::Right);
    assert_eq!(doc.nodes.len(), 4);
    assert_eq!(doc.edges.len(), 3);

    let pedido = class(&doc, "Pedido");
    assert_eq!(pedido.attributes.len(), 3);
    assert_eq!(pedido.operations.len(), 2);
    assert!(pedido.attributes[2].is_static);
    assert_eq!(pedido.operations[1].type_name, "Pedido");

    let repo = class(&doc, "Repositorio");
    assert_eq!(repo.stereotype.as_deref(), Some("interface"));
    assert!(repo.operations[0].is_abstract);

    assert_eq!(doc.edges[1].source.label.as_deref(), Some("1"));
    assert_eq!(doc.edges[2].label.as_deref(), Some("usa"));
}

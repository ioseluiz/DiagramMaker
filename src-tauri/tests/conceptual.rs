//! El modelo conceptual, de ida y vuelta.
//!
//! Un conceptual y un entidad-relacion sin columnas se escriben **igual** en
//! `erDiagram`: entidades y relaciones, nada mas. Lo unico que los distingue
//! al volver a leerlos es la marca en comentario que pone el generador.
//!
//! De ahi que estas pruebas sean sobre todo de ida y vuelta: lo que hay que
//! sostener no es que el texto salga bonito, sino que el texto que sale
//! vuelva a entrar siendo el mismo diagrama y del mismo nivel.
//!
//! La alternativa que se descarto —deducir el nivel de que ninguna entidad
//! declarase atributos— esta cubierta aqui al reves: un `erDiagram` sin
//! marca tiene que seguir siendo un entidad-relacion.

use diagrammaker_lib::emit;
use diagrammaker_lib::model::{DiagramKind, Document, NodeShape};
use diagrammaker_lib::parse::{self, SourceLanguage};

/// Importa y exige que no haya errores.
fn importar(source: &str) -> Document {
    let result = parse::import(SourceLanguage::Mermaid, source, 0);
    assert!(result.ok(), "no deberia fallar: {:#?}", result.diagnostics);
    result.document
}

/// Los codigos de diagnostico que salen al importar.
fn codigos(source: &str) -> Vec<String> {
    parse::import(SourceLanguage::Mermaid, source, 0)
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect()
}

/// Resumen sin identificadores: entidades y relaciones con su cardinalidad.
fn resumen(doc: &Document) -> Vec<String> {
    let nombre = |id: &str| doc.node(id).map(|n| n.label.clone()).unwrap_or_default();

    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|n| format!("entidad {}", n.label))
        .collect();

    out.extend(doc.edges.iter().map(|e| {
        format!(
            "relacion {} {} -- {} {} : {}",
            nombre(&e.source.node),
            e.source.label.as_deref().unwrap_or(""),
            e.target.label.as_deref().unwrap_or(""),
            nombre(&e.target.node),
            e.label.as_deref().unwrap_or(""),
        )
    }));

    out.sort();
    out
}

const CONCEPTUAL: &str = "erDiagram\n\
     \x20 %% diagramMaker: conceptual\n\
     \x20 CLIENTE ||--o{ PEDIDO : \"hace\"\n\
     \x20 PEDIDO ||--|{ LINEA : \"contiene\"\n\
     \x20 PROVEEDOR\n";

/* --- El nivel se marca, no se adivina ------------------------------------ */

#[test]
fn con_la_marca_entra_como_conceptual() {
    assert_eq!(importar(CONCEPTUAL).kind, DiagramKind::Conceptual);
}

#[test]
fn sin_la_marca_sigue_siendo_entidad_relacion() {
    // Lo mismo, palabra por palabra, sin la linea de la marca. Si el nivel se
    // dedujera de que no hay atributos, esto cambiaria de significado — y es
    // texto que el programa ya soportaba antes de que existiera el
    // conceptual.
    let sin_marca: String = CONCEPTUAL
        .lines()
        .filter(|l| !l.contains("diagramMaker"))
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(importar(&sin_marca).kind, DiagramKind::Er);
}

#[test]
fn la_marca_vale_en_cualquier_linea_y_sin_importar_las_mayusculas() {
    for marca in [
        "%% diagramMaker: conceptual",
        "  %% DiagramMaker: Conceptual  ",
    ] {
        let source = format!("erDiagram\n  A ||--o{{ B : \"une\"\n{marca}\n");
        assert_eq!(
            importar(&source).kind,
            DiagramKind::Conceptual,
            "no reconocio `{marca}`"
        );
    }
}

/* --- Lo que se dibuja ------------------------------------------------------ */

#[test]
fn las_entidades_son_rectangulos_sin_tabla() {
    // Es la decision de diseno entera: una entidad conceptual no lleva
    // `ErTable`, y por eso el inspector no le abre el editor de columnas.
    let doc = importar(CONCEPTUAL);

    assert_eq!(doc.nodes.len(), 4);
    for node in &doc.nodes {
        assert_eq!(node.shape, NodeShape::Rectangle, "{}", node.label);
        assert!(node.er().is_none(), "{} llego con tabla", node.label);
    }
}

#[test]
fn la_cardinalidad_y_el_verbo_sobreviven() {
    let doc = importar(CONCEPTUAL);

    assert!(resumen(&doc).contains(&"relacion CLIENTE 1 -- * PEDIDO : hace".to_string()));
}

/* --- Y de vuelta ----------------------------------------------------------- */

#[test]
fn el_codigo_generado_vuelve_a_entrar_igual() {
    let ida = importar(CONCEPTUAL);
    let codigo = emit::emit(SourceLanguage::Mermaid, &ida);
    let vuelta = importar(&codigo);

    assert_eq!(
        vuelta.kind,
        DiagramKind::Conceptual,
        "perdio el nivel:\n{codigo}"
    );
    assert_eq!(
        resumen(&vuelta),
        resumen(&ida),
        "no volvio igual:\n{codigo}"
    );
}

#[test]
fn el_codigo_generado_lleva_la_marca_y_la_cabecera_en_su_sitio() {
    // Mermaid quiere el tipo de diagrama en la primera linea; la marca va
    // detras para que el codigo se siga dibujando en cualquier visor.
    let codigo = emit::emit(SourceLanguage::Mermaid, &importar(CONCEPTUAL));
    let lineas: Vec<&str> = codigo.lines().collect();

    assert_eq!(lineas[0].trim(), "erDiagram");
    assert_eq!(lineas[1].trim(), "%% diagramMaker: conceptual");
}

#[test]
fn una_entidad_suelta_tambien_se_declara() {
    // `PROVEEDOR` no aparece en ninguna relacion: si el generador no la
    // declarase, se perderia al volver a leer.
    let codigo = emit::emit(SourceLanguage::Mermaid, &importar(CONCEPTUAL));

    assert!(
        codigo.contains("PROVEEDOR"),
        "falta la entidad suelta:\n{codigo}"
    );
}

#[test]
fn un_entidad_relacion_con_columnas_no_cambia() {
    // La red que protege lo que ya funcionaba: el generador ahora decide por
    // tipo de diagrama que nodo es una entidad, y el nivel logico tiene que
    // seguir saliendo con sus columnas.
    let source = "erDiagram\n\
         \x20 CLIENTE ||--o{ PEDIDO : \"hace\"\n\
         \x20 CLIENTE {\n\
         \x20   int id PK\n\
         \x20   string correo\n\
         \x20 }\n";

    let doc = importar(source);
    assert_eq!(doc.kind, DiagramKind::Er);

    let codigo = emit::emit(SourceLanguage::Mermaid, &doc);
    assert!(
        codigo.contains("int id PK"),
        "perdio las columnas:\n{codigo}"
    );
    assert!(
        !codigo.contains("diagramMaker"),
        "marco un logico:\n{codigo}"
    );
}

/* --- Lo que no cabe, se dice ----------------------------------------------- */

#[test]
fn un_atributo_en_un_conceptual_avisa() {
    // No tiene donde guardarse. Perderlo en silencio seria peor que decirlo.
    let source = "erDiagram\n\
         \x20 %% diagramMaker: conceptual\n\
         \x20 CLIENTE {\n\
         \x20   int id PK\n\
         \x20 }\n";

    assert!(codigos(source).contains(&"attributeInConceptual".to_string()));
}

#[test]
fn un_alias_en_un_conceptual_avisa() {
    // `PEDIDO["Pedido de venta"]` guarda el identificador en la tabla, y aqui
    // no hay tabla. Se dibuja el texto y se avisa de lo que se pierde.
    let source = "erDiagram\n\
         \x20 %% diagramMaker: conceptual\n\
         \x20 PEDIDO[\"Pedido de venta\"]\n";

    assert!(codigos(source).contains(&"aliasIgnored".to_string()));

    let doc = importar(source);
    assert_eq!(doc.nodes[0].label, "Pedido de venta");
}

#[test]
fn un_nombre_con_espacios_va_y_vuelve_sin_necesitar_alias() {
    // Es lo que hace que perder el alias no sea grave.
    let source = "erDiagram\n\
         \x20 %% diagramMaker: conceptual\n\
         \x20 \"Pedido de venta\" ||--o{ LINEA : \"contiene\"\n";

    let ida = importar(source);
    let vuelta = importar(&emit::emit(SourceLanguage::Mermaid, &ida));

    assert_eq!(resumen(&vuelta), resumen(&ida));
    assert!(vuelta.nodes.iter().any(|n| n.label == "Pedido de venta"));
}

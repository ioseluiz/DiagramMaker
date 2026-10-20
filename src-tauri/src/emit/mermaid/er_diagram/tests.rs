use super::*;

use crate::model::DiagramKind;
use crate::parse::mermaid as parser;

fn import(source: &str) -> Document {
    let result = parser::import(source, 0);
    assert!(
        result.ok(),
        "la importacion fallo: {:#?}",
        result.diagnostics
    );
    assert_eq!(result.document.kind, DiagramKind::Er);
    result.document
}

/// Importa un cuerpo, poniendole la cabecera.
fn doc(source: &str) -> Document {
    import(&format!("erDiagram\n{source}"))
}

/// El codigo generado, sin la cabecera ni las lineas en blanco.
fn code(doc: &Document) -> String {
    emit(doc)
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim_start_matches(' '))
        .collect::<Vec<_>>()
        .join("\n")
}

fn name(doc: &Document, id: &str) -> String {
    doc.node(id).map(|n| n.label.clone()).unwrap_or_default()
}

/// Resumen del diagrama sin ids: entidades con sus columnas y relaciones con
/// su cardinalidad.
fn summary(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .filter_map(|n| {
            let table = n.er()?;
            let columns: Vec<String> = table
                .columns
                .iter()
                .map(|c| format!("{} {} {}", c.data_type, c.name, c.badges().join("+")))
                .collect();
            Some(format!(
                "{} alias={:?} [{}]",
                n.label,
                table.alias,
                columns.join(", ")
            ))
        })
        .collect();
    out.sort();

    let mut edges: Vec<String> = doc
        .edges
        .iter()
        .map(|e| {
            format!(
                "{} {} -- {} {} : {} {}",
                e.source.label.clone().unwrap_or_default(),
                name(doc, &e.source.node),
                name(doc, &e.target.node),
                e.target.label.clone().unwrap_or_default(),
                e.label.clone().unwrap_or_default(),
                if e.style.dashed == Some(true) {
                    "punteada"
                } else {
                    "continua"
                }
            )
        })
        .collect();
    edges.sort();
    out.extend(edges);

    out
}

/// Importar, generar y volver a importar tiene que dar el mismo diagrama.
fn round_trip(source: &str) -> (Document, Document) {
    let first = doc(source);
    let generated = emit(&first);
    let second = parser::import(&generated, 0);

    assert!(
        second.ok(),
        "el codigo generado no vuelve a entrar:\n{generated}\n{:#?}",
        second.diagnostics
    );

    assert_eq!(
        summary(&first),
        summary(&second.document),
        "distinto diagrama.\nGenerado:\n{generated}"
    );

    (first, second.document)
}

/* --- Lo basico ------------------------------------------------------------- */

#[test]
fn un_documento_vacio_genera_solo_la_cabecera() {
    let empty = Document::new("d", DiagramKind::Er, 0);

    assert_eq!(emit(&empty), "erDiagram\n");
}

#[test]
fn una_relacion_sale_con_su_cardinalidad_y_su_etiqueta() {
    assert_eq!(
        code(&doc("CLIENTE ||--o{ PEDIDO : coloca")),
        "CLIENTE ||--o{ PEDIDO : \"coloca\""
    );
}

#[test]
fn una_entidad_con_atributos_sale_en_bloque() {
    assert_eq!(
        code(&doc("CLIENTE {\n  int id PK\n  string nombre\n}")),
        "CLIENTE {\nint id PK\nstring nombre\n}"
    );
}

#[test]
fn una_entidad_sin_relaciones_ni_atributos_se_declara_igual() {
    // Si no se escribiera, desapareceria del diagrama.
    assert_eq!(code(&doc("CLIENTE")), "CLIENTE");
}

#[test]
fn una_entidad_sin_atributos_pero_con_relacion_no_repite_su_nombre() {
    assert_eq!(
        code(&doc("CLIENTE ||--o{ PEDIDO : coloca")),
        "CLIENTE ||--o{ PEDIDO : \"coloca\""
    );
}

#[test]
fn conserva_el_titulo() {
    assert!(emit(&doc("title Ventas\nCLIENTE")).contains("title Ventas"));
}

#[test]
fn conserva_la_direccion_cuando_no_es_la_de_por_defecto() {
    let generated = emit(&doc("direction BT\nCLIENTE"));

    assert!(generated.contains("direction BT"), "{generated}");
}

/* --- Cardinalidad ----------------------------------------------------------- */

#[test]
fn las_cuatro_combinaciones_sobreviven() {
    round_trip(
        "A ||--|| B : a\n\
         C ||--o{ D : b\n\
         E }o--|| F : c\n\
         G }o--o{ H : d",
    );
}

#[test]
fn los_simbolos_opcionales_se_normalizan_a_los_obligatorios() {
    // `|o` y `}|` dicen algo que el documento no guarda; al volver a salir
    // son `||` y `}o`. La perdida esta documentada y es la misma en toda la
    // aplicacion.
    assert_eq!(code(&doc("A |o--|{ B : x")), "A ||--o{ B : \"x\"",);
}

#[test]
fn la_relacion_punteada_sigue_punteada() {
    assert_eq!(code(&doc("A ||..o{ B : x")), "A ||..o{ B : \"x\"");
}

/* --- Atributos --------------------------------------------------------------- */

#[test]
fn las_marcas_de_clave_sobreviven() {
    round_trip(
        "PEDIDO {\n\
         \x20 int numero PK\n\
         \x20 int cliente_id FK\n\
         \x20 string referencia UK\n\
         }",
    );
}

#[test]
fn una_columna_que_es_clave_primaria_y_ajena_sale_con_las_dos() {
    assert_eq!(
        code(&doc("A {\n  int id PK, FK\n}")),
        "A {\nint id PK, FK\n}"
    );
}

#[test]
fn el_comentario_sobrevive() {
    round_trip("A {\n  int id PK \"el numero\"\n}");
}

#[test]
fn una_columna_sin_tipo_sale_con_uno_que_lo_dice() {
    // Mermaid exige dos palabras; una sola rompe el dibujo entero. Antes que
    // inventar un tipo o perder la columna, se dice que no lo tiene.
    use crate::model::{ErColumn, ErTable, Node};

    let mut document = Document::new("d", DiagramKind::Er, 0);
    document.nodes.push(Node::table(
        "t1",
        "CLIENTE",
        ErTable {
            columns: vec![ErColumn::new("c1", "nombre")],
            ..Default::default()
        },
    ));

    assert_eq!(code(&document), "CLIENTE {\nsin_tipo nombre\n}");
}

#[test]
fn un_nombre_de_atributo_con_espacios_se_junta_con_guion_bajo() {
    use crate::model::{ErColumn, ErTable, Node};

    let mut document = Document::new("d", DiagramKind::Er, 0);
    document.nodes.push(Node::table(
        "t1",
        "CLIENTE",
        ErTable {
            columns: vec![ErColumn::new("c1", "fecha de alta").with_type("date")],
            ..Default::default()
        },
    ));

    assert_eq!(code(&document), "CLIENTE {\ndate fecha_de_alta\n}");
}

/* --- Nombres ------------------------------------------------------------------ */

#[test]
fn un_nombre_con_espacios_sale_entrecomillado() {
    round_trip("\"Cliente final\" ||--o{ PEDIDO : coloca");
}

#[test]
fn un_alias_sobrevive_la_ida_y_vuelta() {
    round_trip("PEDIDO[\"Pedido de venta\"] {\n  int numero PK\n}");
}

#[test]
fn un_nombre_con_guiones_no_se_entrecomilla() {
    assert_eq!(
        code(&doc("PEDIDO ||--|{ LINEA-DE-PEDIDO : contiene")),
        "PEDIDO ||--o{ LINEA-DE-PEDIDO : \"contiene\""
    );
}

/* --- Entre los dos idiomas ------------------------------------------------------ */

#[test]
fn una_relacion_entre_columnas_pierde_el_puerto_pero_no_la_relacion() {
    // DBML sabe de que columna a que columna va; Mermaid solo de que entidad
    // a que entidad. La relacion llega entera, sin el puerto.
    let from_dbml = crate::parse::dbml::import(
        "Table clientes {\n  id integer [pk]\n}\n\
         Table pedidos {\n  id integer [pk]\n  cliente_id integer\n}\n\
         Ref: pedidos.cliente_id > clientes.id",
        0,
    );
    assert!(from_dbml.ok(), "{:#?}", from_dbml.diagnostics);

    let generated = emit(&from_dbml.document);
    let back = parser::import(&generated, 0);
    assert!(back.ok(), "{generated}\n{:#?}", back.diagnostics);

    assert_eq!(back.document.nodes.len(), 2);
    assert_eq!(back.document.edges.len(), 1);
    assert_eq!(back.document.edges[0].source.port, None);
    assert_eq!(
        back.document.edges[0].source.label.as_deref(),
        Some("*"),
        "{generated}"
    );
}

#[test]
fn el_mismo_esquema_en_dbml_y_en_mermaid_da_las_mismas_tablas() {
    let from_dbml = crate::parse::dbml::import(
        "Table clientes {\n  id integer [pk]\n  nombre varchar\n}",
        0,
    );
    let from_mermaid = parser::import(
        "erDiagram\n  clientes {\n    integer id PK\n    varchar nombre\n  }",
        0,
    );

    assert!(from_dbml.ok(), "{:#?}", from_dbml.diagnostics);
    assert!(from_mermaid.ok(), "{:#?}", from_mermaid.diagnostics);

    let (a, b) = (from_dbml.document, from_mermaid.document);
    assert_eq!(a.kind, b.kind);
    assert_eq!(summary(&a), summary(&b));
}

/* --- Diagrama completo ---------------------------------------------------------- */

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    round_trip(
        "CLIENTE ||--o{ PEDIDO : coloca\n\
         PEDIDO ||--|{ LINEA : contiene\n\
         CLIENTE }|..|{ DIRECCION : usa\n\
         \n\
         CLIENTE {\n\
         \x20 int id PK\n\
         \x20 string nombre\n\
         }\n\
         PEDIDO {\n\
         \x20 int numero PK\n\
         \x20 int cliente_id FK \"a quien pertenece\"\n\
         \x20 date fecha\n\
         }\n\
         LINEA {\n\
         \x20 int id PK\n\
         \x20 int cantidad\n\
         }",
    );
}

#[test]
fn el_generador_publico_lleva_el_entidad_relacion_a_mermaid() {
    // Entrando por `emit::emit`, que es por donde entra la aplicacion.
    let document = doc("CLIENTE ||--o{ PEDIDO : coloca");
    let generated = crate::emit::emit(crate::emit::SourceLanguage::Mermaid, &document);

    assert!(generated.starts_with("erDiagram"), "{generated}");
    assert!(generated.contains("CLIENTE ||--o{ PEDIDO"), "{generated}");
}

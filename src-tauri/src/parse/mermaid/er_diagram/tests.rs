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
    ok(&format!("erDiagram\n{source}"))
}

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn table<'a>(doc: &'a Document, label: &str) -> &'a ErTable {
    doc.nodes
        .iter()
        .find(|n| n.label == label)
        .unwrap_or_else(|| panic!("no hay ninguna entidad `{label}`"))
        .er()
        .expect("la entidad no tiene tabla")
}

/// Las columnas con sus marcas, para comparar de un vistazo.
fn columns(doc: &Document, label: &str) -> Vec<String> {
    table(doc, label)
        .columns
        .iter()
        .map(|c| {
            let badges = c.badges();
            if badges.is_empty() {
                format!("{} {}", c.data_type, c.name)
            } else {
                format!("{} {} {}", c.data_type, c.name, badges.join("+"))
            }
        })
        .collect()
}

/// Las relaciones con su cardinalidad.
fn relations(doc: &Document) -> Vec<String> {
    doc.edges
        .iter()
        .map(|e| {
            let name = |id: &str| doc.node(id).map(|n| n.label.clone()).unwrap_or_default();
            format!(
                "{} {} -- {} {} : {}",
                e.source.label.clone().unwrap_or_default(),
                name(&e.source.node),
                name(&e.target.node),
                e.target.label.clone().unwrap_or_default(),
                e.label.clone().unwrap_or_default()
            )
        })
        .collect()
}

/* --- Lo basico ------------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_entidad_relacion_y_llega_sin_geometria() {
    let doc = body("CLIENTE ||--o{ PEDIDO : coloca");

    assert_eq!(doc.kind, DiagramKind::Er);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn una_entidad_se_dibuja_como_tabla() {
    use crate::model::NodeShape;

    assert_eq!(body("CLIENTE").nodes[0].shape, NodeShape::Table);
}

#[test]
fn mencionar_una_entidad_en_una_relacion_la_crea() {
    assert_eq!(body("CLIENTE ||--o{ PEDIDO : coloca").nodes.len(), 2);
}

#[test]
fn una_entidad_no_se_duplica_al_nombrarla_dos_veces() {
    let doc = body(
        "CLIENTE ||--o{ PEDIDO : coloca\n\
         CLIENTE ||--o{ DIRECCION : usa",
    );

    assert_eq!(doc.nodes.len(), 3);
}

#[test]
fn lee_el_titulo() {
    assert_eq!(body("title Ventas\nCLIENTE").meta.title, "Ventas");
}

#[test]
fn lee_la_direccion() {
    assert_eq!(
        body("direction LR\nCLIENTE").direction,
        LayoutDirection::Right
    );
}

#[test]
fn se_salta_los_comentarios() {
    assert_eq!(body("%% nada\nCLIENTE\n%% ni esto\nPEDIDO").nodes.len(), 2);
}

/* --- Atributos ------------------------------------------------------------- */

#[test]
fn el_tipo_va_delante_del_nombre() {
    // Al reves que en DBML, que es la otra manera de escribir lo mismo.
    let doc = body("CLIENTE {\n  string nombre\n}");

    assert_eq!(columns(&doc, "CLIENTE"), ["string nombre"]);
}

#[test]
fn lee_las_marcas_de_clave() {
    let doc = body(
        "PEDIDO {\n\
         \x20 int numero PK\n\
         \x20 int cliente_id FK\n\
         \x20 string referencia UK\n\
         }",
    );

    assert_eq!(
        columns(&doc, "PEDIDO"),
        ["int numero PK", "int cliente_id FK", "string referencia U"]
    );
}

#[test]
fn una_clave_primaria_deja_de_admitir_nulos() {
    let doc = body("PEDIDO {\n  int numero PK\n}");

    assert!(!table(&doc, "PEDIDO").columns[0].nullable);
}

#[test]
fn varias_marcas_separadas_por_coma() {
    let doc = body("PEDIDO {\n  int cliente_id PK, FK\n}");

    let column = &table(&doc, "PEDIDO").columns[0];
    assert!(column.primary_key);
    assert!(column.foreign_key);
}

#[test]
fn el_comentario_entrecomillado_se_guarda_como_nota() {
    let doc = body("PEDIDO {\n  int cliente_id FK \"quien lo pidio\"\n}");

    assert_eq!(
        table(&doc, "PEDIDO").columns[0].note.as_deref(),
        Some("quien lo pidio")
    );
}

#[test]
fn un_tipo_con_parentesis_o_corchetes_se_guarda_tal_cual() {
    let doc = body("CLIENTE {\n  varchar(255) nombre\n  string[] etiquetas\n}");

    assert_eq!(
        columns(&doc, "CLIENTE"),
        ["varchar(255) nombre", "string[] etiquetas"]
    );
}

#[test]
fn el_bloque_puede_cerrarse_en_la_linea_del_ultimo_atributo() {
    let doc = body("CLIENTE {\n  string nombre }");

    assert_eq!(columns(&doc, "CLIENTE"), ["string nombre"]);
}

#[test]
fn un_atributo_sin_nombre_es_un_error() {
    let result = parse("erDiagram\nCLIENTE {\n  string\n}");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badAttribute"]);
}

#[test]
fn una_marca_desconocida_es_un_error() {
    let result = parse("erDiagram\nCLIENTE {\n  string nombre XX\n}");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badAttribute"]);
}

#[test]
fn un_bloque_sin_cerrar_es_un_error() {
    let result = parse("erDiagram\nCLIENTE {\n  string nombre");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedEntity"]);
}

#[test]
fn una_llave_de_mas_es_un_error() {
    let result = parse("erDiagram\nCLIENTE\n}");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

/* --- Relaciones ------------------------------------------------------------ */

#[test]
fn lee_la_cardinalidad_y_la_etiqueta() {
    let doc = body("CLIENTE ||--o{ PEDIDO : coloca");

    assert_eq!(relations(&doc), ["1 CLIENTE -- PEDIDO * : coloca"]);
}

#[test]
fn los_cuatro_simbolos_se_redondean_a_uno_o_varios() {
    // Mermaid distingue "cero o uno" de "exactamente uno"; el documento no.
    // Es la perdida que se acepta a cambio de tener una sola cardinalidad en
    // toda la aplicacion.
    let doc = body(
        "A |o--o| B : a\n\
         C ||--|| D : b\n\
         E }o--o{ F : c\n\
         G }|--|{ H : d",
    );

    assert_eq!(
        relations(&doc),
        [
            "1 A -- B 1 : a",
            "1 C -- D 1 : b",
            "* E -- F * : c",
            "* G -- H * : d",
        ]
    );
}

#[test]
fn la_relacion_no_identificadora_se_dibuja_discontinua() {
    let doc = body("CLIENTE ||..o{ PEDIDO : coloca");

    assert_eq!(doc.edges[0].style.dashed, Some(true));
}

#[test]
fn la_identificadora_se_dibuja_continua() {
    assert_eq!(body("A ||--o{ B : x").edges[0].style.dashed, None);
}

#[test]
fn la_etiqueta_puede_ir_entrecomillada() {
    let doc = body("CLIENTE ||--o{ PEDIDO : \"coloca varios\"");

    assert_eq!(doc.edges[0].label.as_deref(), Some("coloca varios"));
}

#[test]
fn una_relacion_sin_etiqueta_tambien_vale() {
    let doc = body("CLIENTE ||--o{ PEDIDO : \"\"");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.edges[0].label, None);
}

#[test]
fn un_nombre_con_guiones_no_confunde_al_buscador_de_relaciones() {
    // `LINEA-DE-PEDIDO` lleva guiones sueltos; el operador son dos guiones
    // con cardinalidades alrededor.
    let doc = body("PEDIDO ||--|{ LINEA-DE-PEDIDO : contiene");

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.nodes[1].label, "LINEA-DE-PEDIDO");
}

#[test]
fn un_nombre_entrecomillado_conserva_los_espacios() {
    let doc = body("\"Cliente final\" ||--o{ PEDIDO : coloca");

    assert_eq!(doc.nodes[0].label, "Cliente final");
}

#[test]
fn un_alias_entre_corchetes_es_lo_que_se_dibuja() {
    let doc = body("PEDIDO[\"Pedido de venta\"] {\n  int numero PK\n}");

    assert_eq!(doc.nodes[0].label, "Pedido de venta");
    assert_eq!(
        table(&doc, "Pedido de venta").alias.as_deref(),
        Some("PEDIDO")
    );
}

#[test]
fn el_alias_alcanza_a_la_entidad_que_ya_existia() {
    // La relacion la crea antes de que el bloque diga como se llama.
    let doc = body(
        "CLIENTE ||--o{ PEDIDO : coloca\n\
         PEDIDO[\"Pedido de venta\"] {\n\
         \x20 int numero PK\n\
         }",
    );

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.nodes[1].label, "Pedido de venta");
    assert_eq!(doc.edges[0].target.node, doc.nodes[1].id);
}

#[test]
fn las_relaciones_llegan_sin_puerto() {
    // En Mermaid la relacion es entre entidades; quien dice cual es la clave
    // ajena es el atributo, con su `FK`.
    let doc = body("CLIENTE ||--o{ PEDIDO : coloca");

    assert_eq!(doc.edges[0].source.port, None);
    assert_eq!(doc.edges[0].target.port, None);
}

#[test]
fn una_relacion_sin_entidad_a_la_izquierda_es_un_error() {
    let result = parse("erDiagram\n||--o{ PEDIDO : coloca");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingRelation"]);
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
erDiagram
    CLIENTE ||--o{ PEDIDO : coloca
    PEDIDO ||--|{ LINEA : contiene
    CLIENTE }|..|{ DIRECCION : usa

    CLIENTE {
        int id PK
        string nombre
        string sector
    }
    PEDIDO {
        int numero PK
        int cliente_id FK "a quien pertenece"
        date fecha
    }
    LINEA {
        int id PK
        int pedido_id FK
        int cantidad
    }
"#);

    assert_eq!(doc.nodes.len(), 4, "tres con bloque y una solo nombrada");
    assert_eq!(doc.edges.len(), 3);

    assert_eq!(
        columns(&doc, "PEDIDO"),
        ["int numero PK", "int cliente_id FK", "date fecha"]
    );
    assert!(table(&doc, "DIRECCION").columns.is_empty());

    assert_eq!(
        relations(&doc),
        [
            "1 CLIENTE -- PEDIDO * : coloca",
            "1 PEDIDO -- LINEA * : contiene",
            "* CLIENTE -- DIRECCION * : usa",
        ]
    );
    assert_eq!(
        doc.edges[2].style.dashed,
        Some(true),
        "la tercera es punteada"
    );
}

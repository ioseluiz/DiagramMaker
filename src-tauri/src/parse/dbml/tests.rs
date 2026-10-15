use super::*;
use crate::model::Severity;

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

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn table<'a>(doc: &'a Document, name: &str) -> &'a ErTable {
    doc.nodes
        .iter()
        .find(|n| n.label == name)
        .unwrap_or_else(|| panic!("no hay ninguna tabla `{name}`"))
        .er()
        .expect("el nodo no es una tabla")
}

/* --- Tablas y columnas -------------------------------------------------- */

#[test]
fn una_tabla_vacia_produce_un_nodo() {
    let doc = ok("Table users {\n}");

    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].label, "users");
    assert_eq!(doc.kind, DiagramKind::Er);
}

#[test]
fn el_documento_importado_llega_sin_geometria() {
    // Colocar las tablas es cosa del auto-layout, que mide el texto.
    let doc = ok("Table users {\n  id integer\n}");

    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
    assert!(doc.needs_layout());
}

#[test]
fn lee_columnas_con_su_tipo() {
    let doc = ok("Table users {\n  id integer\n  email varchar\n}");
    let t = table(&doc, "users");

    assert_eq!(t.columns.len(), 2);
    assert_eq!(t.columns[0].name, "id");
    assert_eq!(t.columns[0].data_type, "integer");
    assert_eq!(t.columns[1].data_type, "varchar");
}

#[test]
fn conserva_el_tamano_del_tipo() {
    let doc = ok("Table users {\n  email varchar(255)\n}");

    assert_eq!(table(&doc, "users").columns[0].data_type, "varchar(255)");
}

#[test]
fn conserva_los_tipos_con_varios_argumentos() {
    let doc = ok("Table m {\n  precio decimal(10,2)\n}");

    assert_eq!(table(&doc, "m").columns[0].data_type, "decimal(10, 2)");
}

#[test]
fn reconoce_los_tipos_de_lista() {
    let doc = ok("Table m {\n  etiquetas text[]\n}");

    assert_eq!(table(&doc, "m").columns[0].data_type, "text[]");
}

#[test]
fn una_columna_sin_tipo_es_valida() {
    let doc = ok("Table m {\n  nombre\n}");

    assert_eq!(table(&doc, "m").columns[0].data_type, "");
}

/* --- Ajustes de columna -------------------------------------------------- */

#[test]
fn pk_marca_clave_primaria_y_quita_la_nulabilidad() {
    let doc = ok("Table users {\n  id integer [pk]\n}");
    let c = &table(&doc, "users").columns[0];

    assert!(c.primary_key);
    assert!(!c.nullable);
}

#[test]
fn acepta_primary_key_escrito_con_dos_palabras() {
    let doc = ok("Table users {\n  id integer [primary key]\n}");

    assert!(table(&doc, "users").columns[0].primary_key);
}

#[test]
fn lee_not_null_unique_e_increment() {
    let doc = ok("Table users {\n  email varchar [not null, unique]\n  n int [increment]\n}");
    let t = table(&doc, "users");

    assert!(!t.columns[0].nullable);
    assert!(t.columns[0].unique);
    assert!(t.columns[1].increment);
}

#[test]
fn lee_el_valor_por_defecto_entre_comillas() {
    let doc = ok("Table users {\n  rol varchar [default: 'user']\n}");

    assert_eq!(
        table(&doc, "users").columns[0].default_value.as_deref(),
        Some("user")
    );
}

#[test]
fn lee_el_valor_por_defecto_como_expresion_sql() {
    let doc = ok("Table users {\n  creado timestamp [default: `now()`]\n}");

    assert_eq!(
        table(&doc, "users").columns[0].default_value.as_deref(),
        Some("`now()`")
    );
}

#[test]
fn lee_la_nota_de_una_columna() {
    let doc = ok("Table users {\n  id int [note: 'identificador']\n}");

    assert_eq!(
        table(&doc, "users").columns[0].note.as_deref(),
        Some("identificador")
    );
}

#[test]
fn un_ajuste_desconocido_avisa_pero_no_rompe() {
    let result = parse("Table users {\n  id int [inventado: 3]\n}");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownSetting"]);
    assert_eq!(result.document.nodes.len(), 1);
}

/* --- Notas, alias y esquema --------------------------------------------- */

#[test]
fn lee_el_alias_de_la_tabla() {
    let doc = ok("Table users as U {\n  id int\n}");

    assert_eq!(table(&doc, "users").alias.as_deref(), Some("U"));
}

#[test]
fn el_alias_sirve_para_referenciar_la_tabla() {
    let doc = ok("Table users as U {\n  id int\n}\n\
         Table posts {\n  user_id int\n}\n\
         Ref: posts.user_id > U.id");

    assert_eq!(doc.edges.len(), 1);
}

#[test]
fn lee_el_esquema_de_la_tabla() {
    let doc = ok("Table public.users {\n  id int\n}");

    assert_eq!(table(&doc, "users").schema.as_deref(), Some("public"));
}

#[test]
fn lee_la_nota_de_la_tabla_en_su_cuerpo() {
    let doc = ok("Table users {\n  id int\n  Note: 'Usuarios del sistema'\n}");

    assert_eq!(
        table(&doc, "users").note.as_deref(),
        Some("Usuarios del sistema")
    );
}

#[test]
fn lee_una_nota_multilinea() {
    let doc = ok("Table users {\n  id int\n  Note: '''\n    Primera\n    Segunda\n  '''\n}");

    assert_eq!(
        table(&doc, "users").note.as_deref(),
        Some("Primera\nSegunda")
    );
}

#[test]
fn el_color_de_cabecera_pasa_a_relleno_del_nodo() {
    let doc = ok("Table users [headercolor: #3498db] {\n  id int\n}");

    assert_eq!(doc.nodes[0].style.fill.as_deref(), Some("#3498db"));
}

/* --- Relaciones ---------------------------------------------------------- */

const DOS_TABLAS: &str =
    "Table users {\n  id integer [pk]\n}\nTable posts {\n  id integer [pk]\n  user_id integer\n}\n";

#[test]
fn crea_la_arista_de_un_ref_suelto() {
    let doc = ok(&format!("{DOS_TABLAS}Ref: posts.user_id > users.id"));

    assert_eq!(doc.edges.len(), 1);
    let e = &doc.edges[0];
    assert_eq!(e.source.node, "t_posts");
    assert_eq!(e.target.node, "t_users");
}

#[test]
fn la_arista_apunta_a_las_columnas_concretas() {
    let doc = ok(&format!("{DOS_TABLAS}Ref: posts.user_id > users.id"));
    let e = &doc.edges[0];

    assert_eq!(e.source.port.as_deref(), Some("t_posts.user_id"));
    assert_eq!(e.target.port.as_deref(), Some("t_users.id"));
}

#[test]
fn muchos_a_uno_pone_las_cardinalidades_en_su_sitio() {
    let doc = ok(&format!("{DOS_TABLAS}Ref: posts.user_id > users.id"));
    let e = &doc.edges[0];

    assert_eq!(e.source.label.as_deref(), Some("*"));
    assert_eq!(e.target.label.as_deref(), Some("1"));
}

#[test]
fn uno_a_muchos_invierte_las_cardinalidades() {
    let doc = ok(&format!("{DOS_TABLAS}Ref: users.id < posts.user_id"));
    let e = &doc.edges[0];

    assert_eq!(e.source.label.as_deref(), Some("1"));
    assert_eq!(e.target.label.as_deref(), Some("*"));
}

#[test]
fn uno_a_uno_y_muchos_a_muchos() {
    let uno = ok(&format!("{DOS_TABLAS}Ref: posts.user_id - users.id"));
    assert_eq!(uno.edges[0].source.label.as_deref(), Some("1"));
    assert_eq!(uno.edges[0].target.label.as_deref(), Some("1"));

    let muchos = ok(&format!("{DOS_TABLAS}Ref: posts.user_id <> users.id"));
    assert_eq!(muchos.edges[0].source.label.as_deref(), Some("*"));
    assert_eq!(muchos.edges[0].target.label.as_deref(), Some("*"));
}

#[test]
fn el_lado_muchos_queda_marcado_como_clave_ajena() {
    let doc = ok(&format!("{DOS_TABLAS}Ref: posts.user_id > users.id"));

    let posts = table(&doc, "posts");
    assert!(posts.column_by_name("user_id").unwrap().foreign_key);

    let users = table(&doc, "users");
    assert!(!users.column_by_name("id").unwrap().foreign_key);
}

#[test]
fn lee_una_relacion_declarada_dentro_de_la_columna() {
    let doc = ok("Table users {\n  id integer [pk]\n}\n\
         Table posts {\n  user_id integer [ref: > users.id]\n}");

    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.edges[0].source.node, "t_posts");
    assert_eq!(doc.edges[0].target.node, "t_users");
}

#[test]
fn lee_un_bloque_ref_con_varias_relaciones() {
    let doc = ok(&format!(
        "{DOS_TABLAS}Ref {{\n  posts.user_id > users.id\n  posts.id - users.id\n}}"
    ));

    assert_eq!(doc.edges.len(), 2);
}

#[test]
fn una_relacion_con_nombre_lo_usa_como_id() {
    let doc = ok(&format!(
        "{DOS_TABLAS}Ref autoria: posts.user_id > users.id"
    ));

    assert_eq!(doc.edges[0].id, "r_autoria");
}

#[test]
fn ignora_los_ajustes_de_la_relacion_sin_romperse() {
    let doc = ok(&format!(
        "{DOS_TABLAS}Ref: posts.user_id > users.id [delete: cascade]"
    ));

    assert_eq!(doc.edges.len(), 1);
}

#[test]
fn las_rutas_con_esquema_resuelven_igual() {
    let doc = ok(&format!(
        "{DOS_TABLAS}Ref: public.posts.user_id > public.users.id"
    ));

    assert_eq!(doc.edges.len(), 1);
}

/* --- Errores y recuperacion ---------------------------------------------- */

#[test]
fn una_relacion_a_una_tabla_inexistente_se_reporta() {
    let result = parse(&format!("{DOS_TABLAS}Ref: posts.user_id > fantasma.id"));

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unknownTable"]);
    // Las tablas buenas siguen ahi.
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn una_relacion_a_una_columna_inexistente_se_reporta() {
    let result = parse(&format!("{DOS_TABLAS}Ref: posts.user_id > users.fantasma"));

    assert_eq!(codes(&result), ["unknownColumn"]);
}

#[test]
fn una_columna_repetida_se_reporta_con_su_linea() {
    let result = parse("Table users {\n  id int\n  id int\n}");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["duplicateColumn"]);
    assert_eq!(result.diagnostics[0].span.unwrap().line, 3);
    assert_eq!(table(&result.document, "users").columns.len(), 1);
}

#[test]
fn una_tabla_repetida_se_reporta() {
    let result = parse("Table users {\n  id int\n}\nTable users {\n  id int\n}");

    assert_eq!(codes(&result), ["duplicateTable"]);
}

#[test]
fn una_tabla_sin_cerrar_se_reporta() {
    let result = parse("Table users {\n  id int");

    assert_eq!(codes(&result), ["unterminatedTable"]);
}

#[test]
fn falta_la_llave_de_apertura() {
    let result = parse("Table users\n  id int\n");

    assert!(codes(&result).contains(&"missingBrace"));
}

#[test]
fn un_error_en_una_tabla_no_impide_leer_las_siguientes() {
    let result = parse(
        "Table rota {\n  id int\n  id int\n}\n\
         Table buena {\n  id int\n}",
    );

    assert_eq!(result.document.nodes.len(), 2);
    assert!(result.document.nodes.iter().any(|n| n.label == "buena"));
}

#[test]
fn un_token_inesperado_se_reporta_una_sola_vez_por_linea() {
    let result = parse("basura suelta aqui\nTable users {\n  id int\n}");

    assert_eq!(codes(&result), ["unexpectedToken"]);
    assert_eq!(result.document.nodes.len(), 1);
}

#[test]
fn un_error_lexico_se_reporta_con_su_posicion() {
    let result = parse("Table users {\n  nota varchar [note: 'sin cerrar\n}");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["lexError"]);
    assert_eq!(result.diagnostics[0].span.unwrap().line, 2);
}

#[test]
fn una_relacion_mal_escrita_se_reporta() {
    let result = parse(&format!("{DOS_TABLAS}Ref: posts.user_id users.id"));

    assert!(codes(&result).contains(&"badRef"));
}

#[test]
fn una_entrada_vacia_produce_un_documento_vacio_y_valido() {
    let result = parse("");

    assert!(result.ok());
    assert!(result.document.nodes.is_empty());
}

#[test]
fn una_entrada_de_solo_comentarios_tampoco_falla() {
    let result = parse("// nada que ver\n/* tampoco */\n");

    assert!(result.ok());
    assert!(result.document.nodes.is_empty());
}

/* --- Bloques auxiliares --------------------------------------------------- */

#[test]
fn los_enums_no_rompen_la_importacion() {
    let doc = ok("Enum estado {\n  borrador\n  publicado\n}\nTable p {\n  s estado\n}");

    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(table(&doc, "p").columns[0].data_type, "estado");
}

#[test]
fn el_bloque_indexes_avisa_de_que_no_se_dibuja() {
    let result = parse("Table users {\n  id int\n  indexes {\n    (id) [pk]\n  }\n}");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredIndexes"]);
    assert_eq!(table(&result.document, "users").columns.len(), 1);
}

#[test]
fn un_tablegroup_se_convierte_en_contenedor() {
    let doc = ok(&format!(
        "{DOS_TABLAS}TableGroup tienda {{\n  users\n  posts\n}}"
    ));

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el grupo");

    assert_eq!(group.label, "tienda");
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn el_contenedor_aparece_antes_que_sus_tablas() {
    let doc = ok(&format!(
        "{DOS_TABLAS}TableGroup tienda {{\n  users\n  posts\n}}"
    ));

    assert_eq!(doc.nodes[0].shape, NodeShape::Package);
}

#[test]
fn un_grupo_con_una_tabla_inexistente_avisa() {
    let result = parse(&format!("{DOS_TABLAS}TableGroup t {{\n  fantasma\n}}"));

    assert_eq!(codes(&result), ["unknownGroupMember"]);
}

#[test]
fn el_proyecto_da_titulo_y_descripcion_al_documento() {
    let doc = ok(
        "Project tienda {\n  database_type: 'PostgreSQL'\n  Note: 'Esquema de la tienda'\n}\n\
         Table users {\n  id int\n}",
    );

    assert_eq!(doc.meta.title, "tienda");
    assert_eq!(
        doc.meta.description.as_deref(),
        Some("Esquema de la tienda")
    );
}

/* --- El documento resultante es coherente -------------------------------- */

#[test]
fn el_documento_importado_pasa_la_validacion() {
    let result = parse(&format!(
        "{DOS_TABLAS}Ref: posts.user_id > users.id\nTableGroup t {{\n  users\n}}"
    ));

    let report = crate::model::validate(&result.document);
    assert!(report.ok, "validacion fallida: {:#?}", report.diagnostics);
}

#[test]
fn los_ids_de_nodo_y_columna_son_unicos() {
    let doc = ok("Table a {\n  id int\n  x int\n}\nTable b {\n  id int\n}");

    let mut ids: Vec<&str> = doc.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort_unstable();
    let total = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), total);

    for node in &doc.nodes {
        let table = node.er().unwrap();
        let mut columns: Vec<&str> = table.columns.iter().map(|c| c.id.as_str()).collect();
        let n = columns.len();
        columns.sort_unstable();
        columns.dedup();
        assert_eq!(columns.len(), n, "columnas repetidas en `{}`", node.label);
    }
}

#[test]
fn un_esquema_realista_se_importa_entero() {
    let source = r#"
Project tienda {
  database_type: 'PostgreSQL'
}

Table users as U {
  id integer [pk, increment]
  email varchar(255) [not null, unique]
  rol varchar [default: 'cliente']
  creado timestamp [default: `now()`]
  Note: 'Clientes registrados'
}

Table orders {
  id integer [pk]
  user_id integer [not null, ref: > U.id]
  total decimal(10,2) [not null]
  estado order_status
}

Table order_lines {
  id integer [pk]
  order_id integer [ref: > orders.id]
  producto varchar(120)
  cantidad integer [default: 1]
}

Enum order_status {
  pendiente
  pagado
  enviado
}

TableGroup ventas {
  orders
  order_lines
}
"#;

    let result = parse(source);
    assert!(result.ok(), "diagnosticos: {:#?}", result.diagnostics);

    let doc = result.document;
    assert_eq!(doc.meta.title, "tienda");
    // 3 tablas + 1 grupo
    assert_eq!(doc.nodes.len(), 4);
    assert_eq!(doc.edges.len(), 2);
    assert_eq!(table(&doc, "users").columns.len(), 4);
    assert!(crate::model::validate(&doc).ok);

    let severities: Vec<Severity> = doc.nodes.iter().map(|_| Severity::Info).collect();
    assert_eq!(severities.len(), 4);
}

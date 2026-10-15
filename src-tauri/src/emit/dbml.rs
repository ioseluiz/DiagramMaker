//! Generador de DBML a partir del documento.
//!
//! Lo que sale tiene que poder volver a entrar por el importador y producir el
//! mismo diagrama. Las pruebas de ida y vuelta son las que sostienen eso.

use std::fmt::Write as _;

use crate::model::{Document, ErColumn, ErTable, Node, NodeShape};

/// Genera el codigo DBML del documento.
pub fn emit(doc: &Document) -> String {
    let mut out = String::new();

    emit_project(doc, &mut out);

    for node in &doc.nodes {
        if let Some(table) = node.er() {
            emit_table(node, table, &mut out);
        }
    }

    emit_refs(doc, &mut out);
    emit_groups(doc, &mut out);

    out.trim_end().to_string() + "\n"
}

fn emit_project(doc: &Document, out: &mut String) {
    let title = doc.meta.title.trim();
    let note = doc.meta.description.as_deref().unwrap_or("").trim();

    // Un titulo que es solo el nombre por defecto del tipo no aporta nada.
    if title.is_empty() || (title == "Entidad-relacion" && note.is_empty()) {
        return;
    }

    let _ = writeln!(out, "Project {} {{", identifier(title));
    if !note.is_empty() {
        let _ = writeln!(out, "  Note: {}", quote(note));
    }
    out.push_str("}\n\n");
}

fn emit_table(node: &Node, table: &ErTable, out: &mut String) {
    let name = match &table.schema {
        Some(schema) => format!("{}.{}", identifier(schema), identifier(&node.label)),
        None => identifier(&node.label),
    };

    let _ = write!(out, "Table {name}");
    if let Some(alias) = &table.alias {
        let _ = write!(out, " as {}", identifier(alias));
    }
    if let Some(color) = &node.style.fill {
        let _ = write!(out, " [headercolor: {color}]");
    }
    out.push_str(" {\n");

    for column in &table.columns {
        emit_column(column, out);
    }

    if let Some(note) = &table.note {
        let _ = writeln!(out, "  Note: {}", quote(note));
    }

    out.push_str("}\n\n");
}

fn emit_column(column: &ErColumn, out: &mut String) {
    let _ = write!(out, "  {}", identifier(&column.name));
    if !column.data_type.is_empty() {
        let _ = write!(out, " {}", column.data_type);
    }

    let mut settings: Vec<String> = Vec::new();
    if column.primary_key {
        settings.push("pk".into());
    }
    if column.increment {
        settings.push("increment".into());
    }
    if column.unique && !column.primary_key {
        settings.push("unique".into());
    }
    // Una clave primaria ya implica `not null`; repetirlo es ruido.
    if !column.nullable && !column.primary_key {
        settings.push("not null".into());
    }
    if let Some(default) = &column.default_value {
        // Van sin comillas las expresiones (que ya traen sus acentos graves
        // del importador), los numeros y los literales del lenguaje. El resto
        // es texto y se entrecomilla.
        let literal = default.starts_with('`')
            || default.parse::<f64>().is_ok()
            || matches!(default.as_str(), "true" | "false" | "null");

        let value = if literal {
            default.clone()
        } else {
            quote(default)
        };
        settings.push(format!("default: {value}"));
    }
    if let Some(note) = &column.note {
        settings.push(format!("note: {}", quote(note)));
    }

    if !settings.is_empty() {
        let _ = write!(out, " [{}]", settings.join(", "));
    }
    out.push('\n');
}

fn emit_refs(doc: &Document, out: &mut String) {
    let mut wrote_any = false;

    for edge in &doc.edges {
        let (Some(source), Some(target)) = (
            column_path(doc, &edge.source.node, edge.source.port.as_deref()),
            column_path(doc, &edge.target.node, edge.target.port.as_deref()),
        ) else {
            continue;
        };

        let op = match (edge.source.label.as_deref(), edge.target.label.as_deref()) {
            (Some("*"), Some("*")) => "<>",
            (Some("*"), _) => ">",
            (_, Some("*")) => "<",
            _ => "-",
        };

        let _ = writeln!(out, "Ref: {source} {op} {target}");
        wrote_any = true;
    }

    if wrote_any {
        out.push('\n');
    }
}

fn emit_groups(doc: &Document, out: &mut String) {
    for group in doc.nodes.iter().filter(|n| n.shape == NodeShape::Package) {
        let members: Vec<String> = doc
            .children_of(Some(&group.id))
            .filter(|n| n.er().is_some())
            .map(|n| identifier(&n.label))
            .collect();

        if members.is_empty() {
            continue;
        }

        let _ = writeln!(out, "TableGroup {} {{", identifier(&group.label));
        for member in members {
            let _ = writeln!(out, "  {member}");
        }
        out.push_str("}\n\n");
    }
}

/// `tabla.columna` para una arista, resolviendo los ids a nombres.
fn column_path(doc: &Document, node_id: &str, port: Option<&str>) -> Option<String> {
    let node = doc.node(node_id)?;
    let table = node.er()?;

    let column = match port {
        Some(port) => table.column(port)?,
        // Sin columna concreta, la clave primaria es la referencia natural.
        None => table.primary_keys().next().or(table.columns.first())?,
    };

    Some(format!(
        "{}.{}",
        identifier(&node.label),
        identifier(&column.name)
    ))
}

/// Entrecomilla si el nombre no es un identificador simple.
fn identifier(name: &str) -> String {
    let simple = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.chars().next().is_some_and(|c| c.is_ascii_digit());

    if simple {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\\\""))
    }
}

/// Texto entre comillas simples, en bloque si tiene saltos de linea.
fn quote(text: &str) -> String {
    if text.contains('\n') {
        format!("'''\n{text}\n'''")
    } else {
        format!("'{}'", text.replace('\'', "\\'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::dbml;

    /// Importa, genera y vuelve a importar. Si el parser y el emisor estan de
    /// acuerdo, los dos documentos coinciden salvo por el id, que es aleatorio.
    fn round_trip(source: &str) -> (Document, Document) {
        let first = dbml::import(source, 0);
        assert!(
            first.ok(),
            "la primera importacion fallo: {:#?}",
            first.diagnostics
        );

        let code = emit(&first.document);
        let second = dbml::import(&code, 0);
        assert!(
            second.ok(),
            "el codigo generado no vuelve a entrar:\n{code}\n{:#?}",
            second.diagnostics
        );

        (first.document, second.document)
    }

    fn assert_equivalent(a: &Document, b: &Document) {
        assert_eq!(a.nodes.len(), b.nodes.len(), "distinto numero de nodos");
        assert_eq!(a.edges.len(), b.edges.len(), "distinto numero de aristas");

        for (left, right) in a.nodes.iter().zip(&b.nodes) {
            assert_eq!(left.label, right.label);
            assert_eq!(left.shape, right.shape);
            assert_eq!(left.er(), right.er(), "tabla `{}`", left.label);
        }

        for (left, right) in a.edges.iter().zip(&b.edges) {
            assert_eq!(left.source.label, right.source.label);
            assert_eq!(left.target.label, right.target.label);
        }
    }

    #[test]
    fn una_tabla_simple_sobrevive_la_ida_y_vuelta() {
        let (a, b) = round_trip("Table users {\n  id integer [pk]\n  email varchar(255)\n}");

        assert_equivalent(&a, &b);
    }

    #[test]
    fn el_codigo_generado_es_legible() {
        let result = dbml::import("Table users {\n  id integer [pk]\n}", 0);

        assert_eq!(
            emit(&result.document),
            "Table users {\n  id integer [pk]\n}\n"
        );
    }

    #[test]
    fn una_clave_primaria_no_repite_not_null() {
        let result = dbml::import("Table t {\n  id int [pk, not null]\n}", 0);

        let code = emit(&result.document);
        assert!(code.contains("[pk]"), "{code}");
        assert!(!code.contains("not null"), "{code}");
    }

    #[test]
    fn conserva_tipos_restricciones_y_valores_por_defecto() {
        let (a, b) = round_trip(
            "Table t {\n  id integer [pk, increment]\n  email varchar(255) [not null, unique]\n  \
             rol varchar [default: 'cliente']\n  creado timestamp [default: `now()`]\n}",
        );

        assert_equivalent(&a, &b);
        let t = b.nodes[0].er().unwrap();
        assert_eq!(t.columns[3].default_value.as_deref(), Some("`now()`"));
    }

    #[test]
    fn conserva_las_relaciones_y_sus_cardinalidades() {
        let (a, b) = round_trip(
            "Table users {\n  id integer [pk]\n}\n\
             Table posts {\n  id integer [pk]\n  user_id integer\n}\n\
             Ref: posts.user_id > users.id",
        );

        assert_equivalent(&a, &b);
        assert_eq!(b.edges[0].source.label.as_deref(), Some("*"));
        assert_eq!(b.edges[0].target.label.as_deref(), Some("1"));
    }

    #[test]
    fn conserva_las_cuatro_cardinalidades() {
        for (op, from, to) in [
            (">", "*", "1"),
            ("<", "1", "*"),
            ("-", "1", "1"),
            ("<>", "*", "*"),
        ] {
            let source = format!(
                "Table a {{\n  id int [pk]\n}}\nTable b {{\n  id int [pk]\n}}\nRef: a.id {op} b.id"
            );
            let (_, b) = round_trip(&source);

            assert_eq!(b.edges[0].source.label.as_deref(), Some(from), "op {op}");
            assert_eq!(b.edges[0].target.label.as_deref(), Some(to), "op {op}");
        }
    }

    #[test]
    fn conserva_alias_esquema_y_notas() {
        let (a, b) = round_trip(
            "Table public.users as U {\n  id int [pk, note: 'identificador']\n  \
             Note: 'Clientes'\n}",
        );

        assert_equivalent(&a, &b);
        let t = b.nodes[0].er().unwrap();
        assert_eq!(t.alias.as_deref(), Some("U"));
        assert_eq!(t.schema.as_deref(), Some("public"));
        assert_eq!(t.note.as_deref(), Some("Clientes"));
    }

    #[test]
    fn conserva_una_nota_multilinea() {
        let (_, b) = round_trip("Table t {\n  id int\n  Note: '''\n    Una\n    Dos\n  '''\n}");

        assert_eq!(b.nodes[0].er().unwrap().note.as_deref(), Some("Una\nDos"));
    }

    #[test]
    fn conserva_los_grupos_como_tablegroup() {
        let (a, b) = round_trip(
            "Table users {\n  id int [pk]\n}\nTable posts {\n  id int [pk]\n}\n\
             TableGroup tienda {\n  users\n  posts\n}",
        );

        assert_equivalent(&a, &b);
        let group = b
            .nodes
            .iter()
            .find(|n| n.shape == NodeShape::Package)
            .unwrap();
        assert_eq!(b.children_of(Some(&group.id)).count(), 2);
    }

    #[test]
    fn conserva_el_titulo_y_la_descripcion_del_proyecto() {
        let (_, b) = round_trip(
            "Project tienda {\n  Note: 'Esquema de la tienda'\n}\nTable t {\n  id int\n}",
        );

        assert_eq!(b.meta.title, "tienda");
        assert_eq!(b.meta.description.as_deref(), Some("Esquema de la tienda"));
    }

    #[test]
    fn entrecomilla_los_nombres_que_lo_necesitan() {
        assert_eq!(identifier("users"), "users");
        assert_eq!(identifier("user_id"), "user_id");
        assert_eq!(identifier("mi tabla"), "\"mi tabla\"");
        assert_eq!(identifier("2fa"), "\"2fa\"");
    }

    #[test]
    fn un_nombre_con_espacios_sobrevive_la_ida_y_vuelta() {
        let (a, b) = round_trip("Table \"mi tabla\" {\n  \"una columna\" int\n}");

        assert_equivalent(&a, &b);
        assert_eq!(b.nodes[0].label, "mi tabla");
    }

    #[test]
    fn un_documento_vacio_genera_codigo_vacio() {
        let doc = Document::new("d", crate::model::DiagramKind::Er, 0);

        assert_eq!(emit(&doc).trim(), "");
    }

    #[test]
    fn un_esquema_realista_sobrevive_la_ida_y_vuelta() {
        let source = r#"
Project tienda {
  Note: 'Esquema de ejemplo'
}

Table users as U {
  id integer [pk, increment]
  email varchar(255) [not null, unique]
  rol varchar [default: 'cliente']
  Note: 'Clientes registrados'
}

Table orders {
  id integer [pk]
  user_id integer [not null]
  total decimal(10,2) [not null]
}

Ref: orders.user_id > U.id

TableGroup ventas {
  orders
}
"#;

        let (a, b) = round_trip(source);
        assert_equivalent(&a, &b);
    }
}

#[cfg(test)]
mod tests_manuales {
    use super::*;
    use crate::model::{DiagramKind, Document, ErColumn, ErTable, Node};
    use crate::parse::dbml;

    /// Reproduce lo que crea el boton de tabla del lienzo: una tabla nueva con
    /// una clave primaria `id` autoincremental.
    fn tabla_nueva(id: &str, nombre: &str) -> Node {
        let mut column = ErColumn::new(format!("{id}.id"), "id")
            .with_type("integer")
            .as_primary_key();
        column.increment = true;

        Node::table(
            id,
            nombre,
            ErTable {
                columns: vec![column],
                ..Default::default()
            },
        )
    }

    #[test]
    fn una_tabla_creada_a_mano_sale_en_el_codigo() {
        let mut doc = Document::new("d", DiagramKind::Er, 0);
        doc.nodes.push(tabla_nueva("t_users", "users"));

        assert_eq!(
            emit(&doc),
            "Table users {\n  id integer [pk, increment]\n}\n"
        );
    }

    #[test]
    fn el_codigo_de_una_tabla_hecha_a_mano_vuelve_a_entrar() {
        let mut doc = Document::new("d", DiagramKind::Er, 0);
        doc.nodes.push(tabla_nueva("t_users", "users"));
        doc.nodes.push(tabla_nueva("t_posts", "posts"));

        let result = dbml::import(&emit(&doc), 0);

        assert!(result.ok(), "diagnosticos: {:#?}", result.diagnostics);
        assert_eq!(result.document.nodes.len(), 2);
    }

    #[test]
    fn una_columna_sin_tipo_produce_codigo_que_dbdiagram_rechazaria() {
        // El lienzo ya no crea columnas sin tipo —`addColumn` pone `varchar`
        // justo por esto—, pero una se puede quedar sin el si el usuario
        // vacia el campo. `validate` lo avisa; aqui se fija que el codigo
        // generado en ese caso es el que es.
        let mut doc = Document::new("d", DiagramKind::Er, 0);
        let mut node = tabla_nueva("t_users", "users");
        if let Some(table) = node.data.as_er_mut() {
            table
                .columns
                .push(ErColumn::new("t_users.nueva", "columna"));
        }
        doc.nodes.push(node);

        let code = emit(&doc);

        assert!(code.contains("\n  columna\n"), "{code}");
        // Y aun asi vuelve a entrar por nuestro importador.
        assert!(dbml::import(&code, 0).ok());
    }
}

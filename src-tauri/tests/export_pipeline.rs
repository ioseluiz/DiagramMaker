//! Prueba de extremo a extremo de la exportacion.
//!
//! Comprueba lo que las pruebas unitarias no pueden: que el SVG generado no
//! solo sea valido, sino que **pinte algo**. Un SVG con un marcador mal
//! referenciado o una fuente que no resuelve se analiza sin errores y produce
//! un PNG en blanco; aqui se cuentan los pixeles.
//!
//! Para dejar los tres archivos en disco y mirarlos:
//!
//! ```sh
//! EXPORT_SAMPLE_DIR=/ruta cargo test --manifest-path src-tauri/Cargo.toml --test export_pipeline
//! ```

use std::collections::HashMap;

use diagrammaker_lib::model::{DiagramKind, Document, Edge, EdgeKind, Node, NodeShape, Point};
use diagrammaker_lib::render::{
    EdgePath, ExportOptions, LabelLayout, LabelPlacement, RenderInput, RenderTheme, TextLine, pdf,
    raster, svg,
};

fn theme() -> RenderTheme {
    RenderTheme {
        background: "#fbfbfc".into(),
        node_fill: "#ffffff".into(),
        node_stroke: "#b6bec9".into(),
        node_text: "#16191d".into(),
        edge_stroke: "#b6bec9".into(),
        edge_text: "#5b6471".into(),
        font_family: "Segoe UI, system-ui, sans-serif".into(),
    }
}

/// Etiqueta de una linea, con un ancho estimado como el que mandaria el
/// navegador.
fn label(text: &str, size: f64, placement: LabelPlacement) -> LabelLayout {
    LabelLayout {
        lines: vec![TextLine {
            text: text.into(),
            width: text.chars().count() as f64 * size * 0.55,
        }],
        font_size: size,
        line_height: size + 4.0,
        placement,
        color: None,
        bold: false,
        italic: false,
    }
}

/// Un diagrama de arquitectura con las ocho formas, anidamiento y varios tipos
/// de relacion: si algo del renderizador esta roto, sale aqui.
fn sample() -> RenderInput {
    let mut doc = Document::new("muestra", DiagramKind::Architecture, 0);
    let mut labels = HashMap::new();

    let shapes: [(&str, &str, NodeShape, f64, f64, f64, f64); 8] = [
        (
            "web",
            "Portal web",
            NodeShape::Rectangle,
            40.0,
            60.0,
            160.0,
            60.0,
        ),
        (
            "api",
            "API REST",
            NodeShape::RoundedRect,
            260.0,
            60.0,
            160.0,
            60.0,
        ),
        (
            "cola",
            "Cola de eventos",
            NodeShape::Ellipse,
            480.0,
            55.0,
            160.0,
            70.0,
        ),
        (
            "ruta",
            "Valido?",
            NodeShape::Diamond,
            260.0,
            190.0,
            120.0,
            80.0,
        ),
        (
            "db",
            "PostgreSQL",
            NodeShape::Cylinder,
            480.0,
            190.0,
            140.0,
            90.0,
        ),
        (
            "nota",
            "Solo lectura",
            NodeShape::Note,
            680.0,
            190.0,
            150.0,
            80.0,
        ),
        (
            "dominio",
            "Dominio",
            NodeShape::Package,
            40.0,
            330.0,
            320.0,
            160.0,
        ),
        (
            "usuario",
            "Operador",
            NodeShape::Actor,
            700.0,
            60.0,
            44.0,
            84.0,
        ),
    ];

    for (id, text, shape, x, y, w, h) in shapes {
        doc.nodes.push(Node::new(id, text, shape).at(x, y, w, h));

        let placement = match shape {
            NodeShape::Actor => LabelPlacement::Below,
            NodeShape::Package => LabelPlacement::Tab,
            _ => LabelPlacement::Center,
        };
        let size = if matches!(shape, NodeShape::Package) {
            12.0
        } else {
            13.0
        };
        labels.insert(id.to_string(), label(text, size, placement));
    }

    // Un nodo dentro del paquete, para probar el anidamiento.
    doc.nodes.push(
        Node::new("entidad", "Pedido", NodeShape::Rectangle)
            .at(70.0, 380.0, 140.0, 50.0)
            .with_parent("dominio"),
    );
    labels.insert(
        "entidad".to_string(),
        label("Pedido", 13.0, LabelPlacement::Center),
    );

    let relations: [(&str, &str, &str, EdgeKind, Option<&str>); 5] = [
        ("e1", "web", "api", EdgeKind::Flow, Some("HTTPS")),
        ("e2", "api", "cola", EdgeKind::Directed, Some("publica")),
        ("e3", "api", "ruta", EdgeKind::Dependency, None),
        ("e4", "ruta", "db", EdgeKind::Association, Some("consulta")),
        ("e5", "dominio", "api", EdgeKind::Composition, None),
    ];

    let mut edges = HashMap::new();
    for (id, from, to, kind, text) in relations {
        let mut edge = Edge::new(id, from, to, kind);
        if let Some(t) = text {
            edge = edge.with_label(t);
            labels.insert(id.to_string(), label(t, 11.0, LabelPlacement::Center));
        }
        doc.edges.push(edge);

        // Trazado recto de borde a borde. En la aplicacion lo calcula React
        // Flow y llega hecho; aqui se aproxima, pero **tiene** que parar en el
        // borde: con lineas de centro a centro las puntas de flecha quedarian
        // tapadas por las cajas y la prueba no veria si se dibujan.
        let a = doc.node(from).and_then(|n| n.geometry).unwrap();
        let b = doc.node(to).and_then(|n| n.geometry).unwrap();
        let start = border_point(&a, b.center());
        let end = border_point(&b, a.center());

        edges.insert(
            id.to_string(),
            EdgePath {
                d: format!("M {} {} L {} {}", start.x, start.y, end.x, end.y),
                label_at: Some(Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0)),
                hops: Vec::new(),
            },
        );
    }

    doc.edges[3].source.label = Some("1".into());
    doc.edges[3].target.label = Some("0..*".into());

    RenderInput {
        document: doc,
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
    }
}

/// Punto del borde de `rect` en direccion a `toward`, partiendo de su centro.
fn border_point(rect: &diagrammaker_lib::model::Rect, toward: Point) -> Point {
    let c = rect.center();
    let dx = toward.x - c.x;
    let dy = toward.y - c.y;

    if dx == 0.0 && dy == 0.0 {
        return c;
    }

    // Cuanto hay que avanzar en cada eje para tocar el borde, y cual se alcanza
    // primero.
    let tx = if dx == 0.0 {
        f64::INFINITY
    } else {
        (rect.width / 2.0) / dx.abs()
    };
    let ty = if dy == 0.0 {
        f64::INFINITY
    } else {
        (rect.height / 2.0) / dy.abs()
    };
    let t = tx.min(ty);

    Point::new(c.x + dx * t, c.y + dy * t)
}

/// Cuantos pixeles difieren del color de fondo.
fn painted_pixels(png: &[u8]) -> usize {
    let decoded = resvg_decode(png);
    let background = [0xfb, 0xfb, 0xfc];

    decoded
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[3] > 0 && px[..3] != background)
        .count()
}

/// Decodifica el PNG volviendo a rasterizar con tiny-skia via `png` interno de
/// resvg no es posible, asi que se usa el decodificador de `tiny_skia`.
fn resvg_decode(png: &[u8]) -> Vec<u8> {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).expect("PNG ilegible");
    pixmap.data().to_vec()
}

#[test]
fn el_svg_contiene_todas_las_formas_y_relaciones() {
    let input = sample();
    let out = svg::render(&input);

    for id in ["Portal web", "API REST", "PostgreSQL", "Operador", "Pedido"] {
        assert!(out.contains(id), "falta la etiqueta `{id}`");
    }

    // Una de cada primitiva: rectangulo, elipse, circulo (cabeza del actor) y
    // trazados.
    assert!(out.contains("<rect"), "sin rectangulos");
    assert!(out.contains("<ellipse"), "sin elipses");
    assert!(out.contains("<circle"), "sin circulos");
    assert!(out.contains("<path"), "sin trazados");

    assert_eq!(out.matches("<g transform=").count(), 9, "faltan nodos");
}

#[test]
fn el_png_no_sale_en_blanco() {
    let svg_text = svg::render(&sample());
    let png = raster::to_png(&svg_text, 1.0).unwrap();

    let painted = painted_pixels(&png);

    // El diagrama ocupa cientos de miles de pixeles; unos pocos miles pintados
    // ya descartan el caso "se analizo pero no se dibujo nada".
    assert!(
        painted > 5_000,
        "solo {painted} pixeles pintados: el PNG esta practicamente vacio"
    );
}

#[test]
fn el_texto_llega_hasta_el_png() {
    // Mismo diagrama con y sin etiquetas: si el texto se dibuja, el segundo
    // tiene que pintar menos pixeles.
    let con_texto = raster::to_png(&svg::render(&sample()), 1.0).unwrap();

    let mut sin = sample();
    sin.labels.clear();
    let sin_texto = raster::to_png(&svg::render(&sin), 1.0).unwrap();

    assert!(
        painted_pixels(&con_texto) > painted_pixels(&sin_texto),
        "las etiquetas no estan llegando al PNG"
    );
}

#[test]
fn los_tres_formatos_salen_del_mismo_svg() {
    let input = sample();
    let svg_text = svg::render(&input);

    let png = raster::to_png(&svg_text, 2.0).unwrap();
    let pdf_bytes = pdf::to_pdf(&svg_text).unwrap();

    assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    assert_eq!(&pdf_bytes[..5], b"%PDF-");

    // El PNG a escala 2 debe medir el doble que el viewBox.
    let view = svg::viewport(&input);
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    assert_eq!(width, (view.width * 2.0) as u32);

    if let Ok(dir) = std::env::var("EXPORT_SAMPLE_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("muestra.svg"), &svg_text).unwrap();
        std::fs::write(dir.join("muestra.png"), &png).unwrap();
        std::fs::write(dir.join("muestra.pdf"), &pdf_bytes).unwrap();
    }
}

#[test]
fn el_fondo_transparente_deja_pixeles_sin_pintar() {
    let mut input = sample();
    input.options.opaque_background = false;

    let png = raster::to_png(&svg::render(&input), 1.0).unwrap();
    let data = resvg_decode(&png);

    let transparentes = data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[3] == 0)
        .count();

    assert!(transparentes > 1_000, "el fondo no quedo transparente");
}

/* --- Entidad-relacion: del codigo DBML al PNG ---------------------------- */

const ESQUEMA_DBML: &str = r#"
Project tienda {
  Note: 'Esquema de ejemplo'
}

Table users as U {
  id integer [pk, increment]
  email varchar(255) [not null, unique]
  rol varchar [default: 'cliente']
  creado timestamp [default: `now()`]
}

Table orders {
  id integer [pk]
  user_id integer [not null, ref: > U.id]
  total decimal(10,2) [not null]
  estado varchar(20)
}

Table order_lines {
  id integer [pk]
  order_id integer [ref: > orders.id]
  producto varchar(120)
  cantidad integer
}
"#;

/// Coloca las tablas en columnas. En la aplicacion lo hace ELK desde el
/// frontend, que es quien mide el texto; aqui basta con una rejilla.
fn place_tables(doc: &mut diagrammaker_lib::model::Document) {
    use diagrammaker_lib::model::shape_geometry::table_height;

    let mut x = 40.0;
    for node in &mut doc.nodes {
        let rows = node.er().map(|t| t.columns.len()).unwrap_or(0);
        node.geometry = Some(diagrammaker_lib::model::Rect::new(
            x,
            60.0,
            240.0,
            table_height(rows),
        ));
        x += 330.0;
    }
}

fn er_sample() -> RenderInput {
    let imported = diagrammaker_lib::parse::dbml::import(ESQUEMA_DBML, 0);
    assert!(
        imported.ok(),
        "el DBML de ejemplo no deberia fallar: {:#?}",
        imported.diagnostics
    );

    let mut doc = imported.document;
    place_tables(&mut doc);

    // Las tablas rotulan sus filas desde el propio documento, asi que solo
    // hacen falta etiquetas para las cardinalidades de las aristas.
    let mut labels = HashMap::new();
    let mut edges = HashMap::new();

    for edge in &doc.edges {
        let a = doc
            .node(&edge.source.node)
            .and_then(|n| n.geometry)
            .unwrap();
        let b = doc
            .node(&edge.target.node)
            .and_then(|n| n.geometry)
            .unwrap();
        let start = border_point(&a, b.center());
        let end = border_point(&b, a.center());

        edges.insert(
            edge.id.clone(),
            EdgePath {
                d: format!("M {} {} L {} {}", start.x, start.y, end.x, end.y),
                label_at: Some(Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0)),
                hops: Vec::new(),
            },
        );
        labels.insert(edge.id.clone(), label("", 11.0, LabelPlacement::Center));
    }

    RenderInput {
        document: doc,
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
    }
}

#[test]
fn el_svg_de_un_er_dibuja_tablas_con_sus_columnas() {
    let out = svg::render(&er_sample());

    for text in ["users", "orders", "order_lines", "email", "varchar(255)"] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    // Las marcas de restriccion se dibujan junto al tipo.
    assert!(out.contains("PK"), "faltan las claves primarias");
    assert!(out.contains("FK"), "faltan las claves ajenas");

    // Regresion: con el texto bien pero sin las reglas horizontales, una tabla
    // parece correcta de un vistazo y no lo esta. `users` tiene 4 columnas:
    // una raya bajo la cabecera y tres entre filas.
    let users = group_of(&out, "translate(40 60)");
    assert_eq!(
        users.matches("<path").count(),
        4,
        "faltan las lineas de separacion de la tabla:
{users}"
    );
}

/// Extrae el `<g>` que empieza por la transformacion indicada.
fn group_of<'a>(svg: &'a str, transform: &str) -> &'a str {
    let start = svg
        .find(&format!(r#"<g transform="{transform}">"#))
        .unwrap_or_else(|| panic!("no hay ningun grupo con `{transform}`"));
    let rest = &svg[start..];
    let end = rest.find("</g>").expect("grupo sin cerrar") + 4;
    &rest[..end]
}

#[test]
fn el_png_de_un_er_no_sale_en_blanco() {
    let png = raster::to_png(&svg::render(&er_sample()), 1.0).unwrap();

    assert!(
        painted_pixels(&png) > 5_000,
        "el PNG del diagrama ER esta practicamente vacio"
    );
}

#[test]
fn el_er_tambien_se_exporta_a_los_tres_formatos() {
    let input = er_sample();
    let svg_text = svg::render(&input);

    let png = raster::to_png(&svg_text, 2.0).unwrap();
    let pdf_bytes = pdf::to_pdf(&svg_text).unwrap();

    assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    assert_eq!(&pdf_bytes[..5], b"%PDF-");

    if let Ok(dir) = std::env::var("EXPORT_SAMPLE_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("er.svg"), &svg_text).unwrap();
        std::fs::write(dir.join("er.png"), &png).unwrap();
    }
}

/* --- ER con grupo, color y texto suelto ---------------------------------- */

/// Comprueba que las tres cosas que se pueden anadir a un ER —encerrar tablas
/// en un grupo, pintarlas y escribir texto suelto— llegan al SVG exportado.
#[test]
fn un_er_con_grupo_color_y_texto_se_exporta_entero() {
    use diagrammaker_lib::model::{Node, NodeShape, Rect};

    let imported = diagrammaker_lib::parse::dbml::import(ESQUEMA_DBML, 0);
    let mut doc = imported.document;
    place_tables(&mut doc);

    // Un grupo que encierra las dos primeras tablas.
    let mut group = Node::new("g1", "Ventas", NodeShape::Package);
    group.geometry = Some(Rect::new(16.0, 14.0, 640.0, 220.0));
    for node in doc.nodes.iter_mut().take(2) {
        node.parent = Some("g1".into());
    }
    doc.nodes.insert(0, group);

    // Color en una tabla y una anotacion suelta.
    doc.nodes[1].style.fill = Some("rgb(37 99 235 / 0.16)".into());

    let mut note = Node::new("txt", "Esquema provisional", NodeShape::Text);
    note.geometry = Some(Rect::new(40.0, 300.0, 220.0, 24.0));
    doc.nodes.push(note);

    let mut labels = HashMap::new();
    labels.insert("g1".to_string(), label("Ventas", 12.0, LabelPlacement::Tab));
    labels.insert(
        "txt".to_string(),
        label("Esquema provisional", 13.0, LabelPlacement::Center),
    );

    let input = RenderInput {
        document: doc,
        labels,
        edges: HashMap::new(),
        theme: theme(),
        options: ExportOptions::default(),
    };

    let out = svg::render(&input);

    assert!(out.contains("Ventas"), "falta el nombre del grupo");
    assert!(out.contains("Esquema provisional"), "falta el texto suelto");
    assert!(
        out.contains("rgb(37 99 235 / 0.16)"),
        "falta el color de la tabla"
    );

    // El texto suelto no dibuja contorno: su grupo va vacio de formas.
    let texto = group_of(&out, "translate(40 300)");
    assert!(
        !texto.contains("<rect") && !texto.contains("<path"),
        "el texto suelto no deberia llevar contorno:\n{texto}"
    );

    assert!(
        painted_pixels(&raster::to_png(&out, 1.0).unwrap()) > 5_000,
        "el PNG salio practicamente vacio"
    );
}

/* --- Saltos en los cruces ------------------------------------------------ */

/// Dos relaciones que se cruzan: la de encima debe dibujar su puente.
#[test]
fn un_cruce_se_marca_con_un_puente() {
    use diagrammaker_lib::model::{Edge, EdgeKind, Node, NodeShape};
    use diagrammaker_lib::render::Hop;

    let mut doc = diagrammaker_lib::model::Document::new("cruce", DiagramKind::Architecture, 0);

    // Cuatro cajas en aspa, de modo que las dos relaciones se crucen en medio.
    for (id, x, y) in [
        ("a", 40.0, 40.0),
        ("b", 460.0, 320.0),
        ("c", 460.0, 40.0),
        ("d", 40.0, 320.0),
    ] {
        doc.nodes
            .push(Node::new(id, id, NodeShape::Rectangle).at(x, y, 160.0, 60.0));
    }

    doc.edges
        .push(Edge::new("e1", "a", "b", EdgeKind::Association));
    doc.edges
        .push(Edge::new("e2", "c", "d", EdgeKind::Association));

    let mut edges = HashMap::new();
    edges.insert(
        "e1".to_string(),
        EdgePath {
            d: "M 200 70 L 460 350".into(),
            label_at: None,
            hops: Vec::new(),
        },
    );
    // La segunda pasa por encima y salta en el cruce.
    edges.insert(
        "e2".to_string(),
        EdgePath {
            d: "M 460 70 L 200 350".into(),
            label_at: None,
            hops: vec![Hop {
                x: 330.0,
                y: 210.0,
                angle: std::f64::consts::PI - 0.82,
            }],
        },
    );

    let input = RenderInput {
        document: doc,
        labels: HashMap::new(),
        edges,
        theme: theme(),
        options: ExportOptions::default(),
    };

    let out = svg::render(&input);
    assert!(
        out.contains(r#"stroke-linecap="butt""#),
        "falta el trazo que borra"
    );

    let png = raster::to_png(&out, 2.0).unwrap();
    assert!(painted_pixels(&png) > 2_000);

    if let Ok(dir) = std::env::var("EXPORT_SAMPLE_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("cruce.png"), &png).unwrap();
    }
}

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
        measures: Default::default(),
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
        measures: Default::default(),
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
        measures: Default::default(),
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
        measures: Default::default(),
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

/* --- Mermaid: del codigo al PNG ------------------------------------------ */

const ARQUITECTURA_MERMAID: &str = r#"
flowchart LR
  usuario([Cliente])

  subgraph frontal[Capa publica]
    web[Portal web]
  end

  subgraph servicios[Servicios]
    api[API REST]
    cola{{Cola de eventos}}
  end

  base[(PostgreSQL)]
  pagos[/Pasarela/]

  usuario --> web
  web -- HTTPS --> api
  api --> base
  api -.-> cola
  api ==> pagos
"#;

/// Coloca los nodos en una rejilla, respetando el anidamiento. En la
/// aplicacion lo hace ELK desde el frontend.
fn place_grid(doc: &mut diagrammaker_lib::model::Document) {
    use diagrammaker_lib::model::{NodeShape, Rect};

    let mut x = 40.0;
    for index in 0..doc.nodes.len() {
        let is_group = doc.nodes[index].shape == NodeShape::Package;
        let y = if doc.nodes[index].parent.is_some() {
            150.0
        } else {
            40.0
        };

        doc.nodes[index].geometry = Some(Rect::new(
            x,
            y,
            if is_group { 240.0 } else { 170.0 },
            if is_group { 200.0 } else { 60.0 },
        ));
        x += 270.0;
    }
}

/// Donde va la etiqueta de cada forma. Espejo de `labelPlacement` del
/// frontend, que es quien lo decide en la aplicacion.
///
/// Un actor es una figura de lineas: su etiqueta no cabe dentro, va debajo.
fn placement_for(shape: diagrammaker_lib::model::NodeShape) -> LabelPlacement {
    use diagrammaker_lib::model::NodeShape;

    match shape {
        NodeShape::Package => LabelPlacement::Tab,
        NodeShape::Actor => LabelPlacement::Below,
        _ => LabelPlacement::Center,
    }
}

/// Coloca, mide y dibuja un diagrama de arquitectura recien importado.
///
/// Hace a mano lo que en la aplicacion hacen ELK y el navegador: repartir las
/// cajas y medir su texto. La rejilla es tosca a proposito —lo que se prueba
/// es que el documento importado llega entero hasta el pixel, no que quede
/// bonito— asi que los hijos de un grupo pueden salirse de su caja.
fn render_arquitectura(mut doc: diagrammaker_lib::model::Document) -> String {
    place_grid(&mut doc);

    let mut labels = HashMap::new();
    for node in &doc.nodes {
        labels.insert(
            node.id.clone(),
            label(&node.label, 13.0, placement_for(node.shape)),
        );
    }

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
        if let Some(text) = &edge.label {
            labels.insert(edge.id.clone(), label(text, 11.0, LabelPlacement::Center));
        }
    }

    svg::render(&RenderInput {
        document: doc,
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
        measures: Default::default(),
    })
}

/// Rasteriza y, si se pidio, deja el PNG donde poder mirarlo.
fn check_png(svg: &str, name: &str) {
    let png = raster::to_png(svg, 1.5).unwrap();
    assert!(painted_pixels(&png) > 5_000);

    if let Ok(dir) = std::env::var("EXPORT_SAMPLE_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.png")), png).unwrap();
    }
}

#[test]
fn un_flowchart_de_mermaid_llega_hasta_el_png() {
    let imported = diagrammaker_lib::parse::mermaid::import(ARQUITECTURA_MERMAID, 0);
    assert!(
        imported.ok(),
        "el Mermaid de ejemplo no deberia fallar: {:#?}",
        imported.diagnostics
    );

    let out = render_arquitectura(imported.document);

    for text in ["Cliente", "Portal web", "API REST", "PostgreSQL", "HTTPS"] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    check_png(&out, "mermaid");
}

/// Mismo diagrama que el de Mermaid, escrito en el otro idioma.
const ARQUITECTURA_PLANTUML: &str = r#"
@startuml
title Plataforma de pagos
left to right direction

actor "Cliente" as cliente

package "Capa publica" {
  [Portal web] as web
  component "API REST" as api
}

package "Datos" {
  database "PostgreSQL" as base
  queue "Eventos" as cola
}

cliente --> web : HTTPS
web --> api
api "1" --> "0..*" base
api ..> cola : publica
@enduml
"#;

#[test]
fn un_diagrama_de_componentes_de_plantuml_llega_hasta_el_png() {
    let imported = diagrammaker_lib::parse::plantuml::import(ARQUITECTURA_PLANTUML, 0);
    assert!(
        imported.ok(),
        "el PlantUML de ejemplo no deberia fallar: {:#?}",
        imported.diagnostics
    );

    let doc = imported.document;
    assert_eq!(doc.meta.title, "Plataforma de pagos");

    let out = render_arquitectura(doc);

    for text in [
        "Cliente",
        "Portal web",
        "API REST",
        "PostgreSQL",
        "Eventos",
        "Capa publica",
        "HTTPS",
        // La cardinalidad de un extremo se dibuja pegada a su caja.
        "0..*",
    ] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    check_png(&out, "plantuml");
}

/* --- Secuencia ----------------------------------------------------------- */

/// Construye un diagrama de secuencia con todo lo que el renderizador sabe
/// dibujar: llamadas, retornos, un mensaje a uno mismo, barras de ejecucion
/// anidadas, un `alt` con dos ramas, una nota y un participante destruido.
fn sequence_sample() -> RenderInput {
    use diagrammaker_lib::model::{
        Block, BlockKind, Branch, Measures, Message, MessageKind, NotePlacement, SequenceNote,
        SequenceScript, Size, Step,
    };

    let participants = [
        ("cliente", "Cliente"),
        ("portal", "Portal web"),
        ("api", "API de pedidos"),
        ("pago", "Pasarela"),
    ];

    let mut doc = Document::new("secuencia", DiagramKind::Sequence, 0);
    let mut labels = HashMap::new();
    let mut measures = Measures::default();

    for (id, nombre) in participants {
        doc.nodes.push(Node::new(id, nombre, NodeShape::Rectangle));
        labels.insert(id.to_string(), label(nombre, 13.0, LabelPlacement::Center));
        measures.headers.insert(
            id.to_string(),
            Size::new(nombre.chars().count() as f64 * 13.0 * 0.55 + 28.0, 40.0),
        );
    }

    /// Un mensaje con su texto, registrando de paso lo que mediria.
    fn step(
        labels: &mut HashMap<String, LabelLayout>,
        measures: &mut diagrammaker_lib::model::Measures,
        id: &str,
        from: &str,
        to: &str,
        kind: MessageKind,
        text: &str,
    ) -> Message {
        if !text.is_empty() {
            let measured = label(text, 11.0, LabelPlacement::Center);
            measures.texts.insert(
                id.to_string(),
                Size::new(measured.width(), measured.height()),
            );
            labels.insert(id.to_string(), measured);
        }
        Message::new(id, from, to, kind).with_text(text)
    }

    macro_rules! msg {
        ($id:expr, $from:expr, $to:expr, $kind:expr, $text:expr) => {
            step(&mut labels, &mut measures, $id, $from, $to, $kind, $text)
        };
    }

    let m1 = msg!("m1", "cliente", "portal", MessageKind::Sync, "comprar").activating();
    let m2 = msg!("m2", "portal", "api", MessageKind::Sync, "POST /pedidos").activating();
    let m3 = msg!("m3", "api", "api", MessageKind::Sync, "validar carrito");
    let m4 = msg!("m4", "api", "pago", MessageKind::Sync, "cobrar").activating();
    let m5 = msg!("m5", "pago", "api", MessageKind::Reply, "autorizado").deactivating();
    let m6 = msg!("m6", "api", "portal", MessageKind::Reply, "201 Created").deactivating();
    let m7 = msg!("m7", "api", "portal", MessageKind::Reply, "402 Rechazado").deactivating();
    let m8 = msg!(
        "m8",
        "portal",
        "cliente",
        MessageKind::Reply,
        "confirmacion"
    )
    .deactivating();
    let m9 = msg!("m9", "api", "pago", MessageKind::Destroy, "cerrar sesion");

    // Una nota sobre dos participantes.
    let nota = SequenceNote {
        id: "n1".into(),
        placement: NotePlacement::Over,
        participants: vec!["portal".into(), "api".into()],
        text: "todo sobre TLS".into(),
    };
    let medida = label(&nota.text, 11.0, LabelPlacement::Center);
    measures
        .texts
        .insert("n1".into(), Size::new(medida.width(), medida.height()));
    labels.insert("n1".into(), medida);

    // Rotulos de los marcos: los compone el navegador, igual que el resto.
    for (id, texto) in [("b1", "alt [pago autorizado]"), ("r2", "[rechazado]")] {
        labels.insert(id.to_string(), label(texto, 11.0, LabelPlacement::Center));
    }

    let alt = Step::Block(Block {
        id: "b1".into(),
        kind: BlockKind::Alt,
        branches: vec![
            Branch::new("r1", "pago autorizado").with_steps(vec![
                Step::Message(m4),
                Step::Message(m5),
                Step::Message(m6),
            ]),
            Branch::new("r2", "rechazado").with_steps(vec![Step::Message(m7)]),
        ],
    });

    doc.script = Some(SequenceScript {
        steps: vec![
            Step::Message(m1),
            Step::Note(nota),
            Step::Message(m2),
            Step::Message(m3),
            alt,
            Step::Message(m8),
            Step::Message(m9),
        ],
    });
    doc.meta.title = "Compra de un pedido".into();

    RenderInput {
        document: doc,
        labels,
        edges: HashMap::new(),
        theme: theme(),
        options: ExportOptions::default(),
        measures,
    }
}

#[test]
fn un_diagrama_de_secuencia_llega_hasta_el_png() {
    let input = sequence_sample();
    let out = svg::render(&input);

    // Participantes, mensajes y rotulos de marco.
    for text in [
        "Cliente",
        "Portal web",
        "API de pedidos",
        "Pasarela",
        "comprar",
        "validar carrito",
        "autorizado",
        "alt [pago autorizado]",
        "[rechazado]",
        "todo sobre TLS",
    ] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    // Lineas de vida punteadas, barras de ejecucion y la cruz del destruido.
    assert!(
        out.contains(r#"stroke-dasharray="4 4""#),
        "faltan las lineas de vida"
    );
    assert!(out.contains("dm-arrow-filled"), "faltan las llamadas");
    assert!(out.contains("dm-arrow-open"), "faltan los retornos");

    check_png(&out, "secuencia");
}

#[test]
fn el_area_de_una_secuencia_sale_del_reparto_y_no_del_documento() {
    // Los nodos de una secuencia no tienen geometria: si el area se calculara
    // como en un grafo, saldria vacia.
    let input = sequence_sample();
    let view = svg::viewport(&input);

    assert!(view.width > 400.0, "{view:?}");
    assert!(view.height > 300.0, "{view:?}");
}

#[test]
fn una_secuencia_vacia_no_revienta_la_exportacion() {
    let mut input = sequence_sample();
    input.document.nodes.clear();
    input.document.script = Some(Default::default());

    let svg = svg::render(&input);

    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
}

/// Mismo diagrama que `sequence_sample`, escrito en Mermaid.
const SECUENCIA_MERMAID: &str = r#"
sequenceDiagram
  title Compra de un pedido
  actor Cliente
  participant web as Portal web
  participant api as API de pedidos
  participant pago as Pasarela

  Cliente->>+web: comprar
  Note over web,api: todo sobre TLS
  web->>+api: POST /pedidos
  api->>api: validar carrito
  alt pago autorizado
    api->>+pago: cobrar
    pago-->>-api: autorizado
    api-->>-web: 201 Created
  else rechazado
    api-->>web: 402 Rechazado
  end
  web-->>-Cliente: confirmacion
  destroy pago
  api->>pago: cerrar sesion
"#;

#[test]
fn una_secuencia_de_mermaid_llega_hasta_el_png() {
    use diagrammaker_lib::model::{Measures, Size};

    let imported = diagrammaker_lib::parse::mermaid::import(SECUENCIA_MERMAID, 0);
    assert!(
        imported.ok(),
        "el Mermaid de ejemplo no deberia fallar: {:#?}",
        imported.diagnostics
    );

    let doc = imported.document;
    assert_eq!(doc.meta.title, "Compra de un pedido");

    // Medir es cosa del navegador; aqui se estima igual que en el resto de
    // esta prueba, que es lo que basta para comprobar que llega al pixel.
    let mut measures = Measures::default();
    let mut labels = HashMap::new();

    for node in &doc.nodes {
        let medida = label(&node.label, 13.0, placement_for(node.shape));
        measures
            .headers
            .insert(node.id.clone(), Size::new(medida.width() + 28.0, 40.0));
        labels.insert(node.id.clone(), medida);
    }

    let script = doc.script.clone().unwrap_or_default();
    for step in script.walk() {
        use diagrammaker_lib::model::Step;

        let (id, text) = match step {
            Step::Message(m) if !m.text.is_empty() => (m.id.clone(), m.text.clone()),
            Step::Note(n) => (n.id.clone(), n.text.clone()),
            Step::Block(b) => {
                // El rotulo del marco no empuja la geometria, solo se dibuja.
                let caption = match b.branches.first() {
                    Some(branch) if !branch.label.is_empty() => {
                        format!("{} [{}]", b.kind.tag(), branch.label)
                    }
                    _ => b.kind.tag().to_string(),
                };
                labels.insert(b.id.clone(), label(&caption, 11.0, LabelPlacement::Center));

                for branch in b.branches.iter().skip(1) {
                    labels.insert(
                        branch.id.clone(),
                        label(&format!("[{}]", branch.label), 11.0, LabelPlacement::Center),
                    );
                }
                continue;
            }
            _ => continue,
        };

        let medida = label(&text, 11.0, LabelPlacement::Center);
        measures
            .texts
            .insert(id.clone(), Size::new(medida.width(), medida.height()));
        labels.insert(id, medida);
    }

    let out = svg::render(&RenderInput {
        document: doc,
        labels,
        edges: HashMap::new(),
        theme: theme(),
        options: ExportOptions::default(),
        measures,
    });

    for text in [
        "Cliente",
        "Portal web",
        "API de pedidos",
        "comprar",
        "validar carrito",
        "alt [pago autorizado]",
        "[rechazado]",
        "todo sobre TLS",
        "cerrar sesion",
    ] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    check_png(&out, "secuencia-mermaid");
}

/// El mismo diagrama que `SECUENCIA_MERMAID`, escrito en PlantUML.
const SECUENCIA_PLANTUML: &str = r#"
@startuml
title Compra de un pedido
actor Cliente
participant "Portal web" as web
participant "API de pedidos" as api
participant "Pasarela" as pago

Cliente -> web ++ : comprar
note over web, api : todo sobre TLS
web -> api ++ : POST /pedidos
api -> api : validar carrito
alt pago autorizado
  api -> pago ++ : cobrar
  pago --> api -- : autorizado
  api --> web -- : 201 Created
else rechazado
  api --> web : 402 Rechazado
end
web --> Cliente -- : confirmacion
api -> pago !! : cerrar sesion
@enduml
"#;

/// Resumen de un guion, sin ids: lo que tiene que coincidir entre idiomas.
fn resumen(steps: &[diagrammaker_lib::model::Step]) -> Vec<String> {
    use diagrammaker_lib::model::Step;

    steps
        .iter()
        .map(|step| match step {
            Step::Message(m) => format!(
                "msg {:?} {} {}{}",
                m.kind,
                m.text,
                if m.activate { "+" } else { "" },
                if m.deactivate { "-" } else { "" }
            ),
            Step::Note(n) => format!("note {:?} x{}", n.placement, n.participants.len()),
            Step::Block(b) => format!(
                "block {:?} [{}]",
                b.kind,
                b.branches
                    .iter()
                    .map(|r| format!("{}:{}", r.label, resumen(&r.steps).join("|")))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        })
        .collect()
}

#[test]
fn el_mismo_diagrama_en_mermaid_y_en_plantuml_da_el_mismo_documento() {
    // Es la prueba que de verdad valida los dos importadores: no que cada
    // uno lea algo, sino que los dos lean lo mismo.
    let de_mermaid = diagrammaker_lib::parse::mermaid::import(SECUENCIA_MERMAID, 0);
    let de_plantuml = diagrammaker_lib::parse::plantuml::import(SECUENCIA_PLANTUML, 0);

    assert!(de_mermaid.ok(), "{:#?}", de_mermaid.diagnostics);
    assert!(de_plantuml.ok(), "{:#?}", de_plantuml.diagnostics);

    let (a, b) = (de_mermaid.document, de_plantuml.document);

    assert_eq!(a.kind, b.kind);
    assert_eq!(a.meta.title, b.meta.title);
    assert_eq!(
        a.nodes
            .iter()
            .map(|n| (n.label.as_str(), n.shape))
            .collect::<Vec<_>>(),
        b.nodes
            .iter()
            .map(|n| (n.label.as_str(), n.shape))
            .collect::<Vec<_>>(),
        "distintos participantes"
    );

    assert_eq!(
        resumen(&a.script.unwrap_or_default().steps),
        resumen(&b.script.unwrap_or_default().steps),
        "distinto guion"
    );
}

/* --- Clases -------------------------------------------------------------- */

/// Un diagrama de clases con todo lo que la caja sabe dibujar: estereotipo,
/// las cuatro visibilidades, miembros estaticos y abstractos, y
/// compartimentos vacios.
fn class_sample() -> RenderInput {
    use diagrammaker_lib::model::shape_geometry::class_height;
    use diagrammaker_lib::model::{ClassBox, ClassMember, NodeData, Rect, Visibility};

    let mut doc = Document::new("clases", DiagramKind::Class, 0);
    let mut labels = HashMap::new();

    let pedido = ClassBox::new()
        .with_attributes(vec![
            ClassMember::new("a1", "id")
                .with_visibility(Visibility::Private)
                .with_type("int"),
            ClassMember::new("a2", "total")
                .with_visibility(Visibility::Private)
                .with_type("Decimal"),
            ClassMember::new("a3", "creados")
                .with_visibility(Visibility::Protected)
                .with_type("int"),
        ])
        .with_operations(vec![
            ClassMember::new("o1", "confirmar"),
            ClassMember::new("o2", "buscar")
                .with_parameters("id: int")
                .with_type("Pedido"),
            ClassMember::new("o3", "borrar")
                .with_visibility(Visibility::Package)
                .with_parameters("id: int"),
        ]);

    let repositorio = ClassBox::new()
        .with_stereotype("interface")
        .with_operations(vec![
            ClassMember::new("o4", "guardar").with_parameters("p: Pedido"),
            ClassMember::new("o5", "todos").with_type("List<Pedido>"),
        ]);

    let vacia = ClassBox::new();

    let cajas = [
        ("pedido", "Pedido", pedido, 40.0),
        ("repo", "RepositorioPedidos", repositorio, 300.0),
        ("vacia", "Marcador", vacia, 560.0),
    ];

    for (id, nombre, box_, x) in cajas {
        let stereotype = box_.has_stereotype();
        let height = class_height(stereotype, box_.attributes.len(), box_.operations.len());

        let mut node = Node::new(id, nombre, NodeShape::Class);
        node.geometry = Some(Rect::new(x, 40.0, 230.0, height));
        node.data = NodeData::Class(box_);
        doc.nodes.push(node);

        labels.insert(id.to_string(), label(nombre, 13.0, LabelPlacement::Center));
    }

    // El tercer miembro de `Pedido` es estatico y la primera operacion de la
    // interfaz, abstracta: son los dos adornos que el dibujo tiene que
    // distinguir.
    if let Some(data) = doc.nodes[0].data.as_class_mut() {
        data.attributes[2].is_static = true;
    }
    if let Some(data) = doc.nodes[1].data.as_class_mut() {
        data.operations[0].is_abstract = true;
    }

    doc.edges
        .push(Edge::new("e1", "pedido", "repo", EdgeKind::Dependency).with_label("usa"));

    let mut edges = HashMap::new();
    edges.insert(
        "e1".to_string(),
        EdgePath {
            d: "M 270 120 L 300 120".into(),
            label_at: Some(Point::new(285.0, 120.0)),
            hops: Vec::new(),
        },
    );
    labels.insert("e1".to_string(), label("usa", 11.0, LabelPlacement::Center));

    RenderInput {
        document: doc,
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
        measures: Default::default(),
    }
}

#[test]
fn un_diagrama_de_clases_llega_hasta_el_png() {
    let input = class_sample();
    let out = svg::render(&input);

    for text in [
        "Pedido",
        "RepositorioPedidos",
        "- id: int",
        "# creados: int",
        "+ confirmar()",
        "+ buscar(id: int): Pedido",
        "~ borrar(id: int)",
        "+ todos(): List&lt;Pedido&gt;",
        "interface",
    ] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    // Lo estatico va subrayado y lo abstracto en cursiva.
    assert!(
        out.contains(r#"text-decoration="underline""#),
        "falta el subrayado"
    );
    assert!(out.contains(r#"font-style="italic""#), "falta la cursiva");

    // El nombre una sola vez: la caja lo rotula ella, y si ademas pasara por
    // la etiqueta medida saldria otra vez centrado en el medio.
    assert_eq!(
        out.matches(">Pedido<").count(),
        1,
        "el nombre sale repetido"
    );

    check_png(&out, "clases");
}

#[test]
fn una_clase_sin_miembros_sigue_teniendo_sus_tres_compartimentos() {
    use diagrammaker_lib::model::shape_geometry::class_primitives;

    // Dos separadores: lo que distingue una clase de un rectangulo con
    // texto dentro.
    let lineas = class_primitives(false, 0, 200.0, 50.0)
        .iter()
        .filter(|p| matches!(p, diagrammaker_lib::model::Primitive::Line { .. }))
        .count();

    assert_eq!(lineas, 2);
}

/// Diagrama de clases escrito en Mermaid.
const CLASES_MERMAID: &str = r#"
classDiagram
  direction LR

  class Pedido {
    -int id
    -Decimal total
    +int creados$
    +confirmar()
    +buscar(int id) Pedido
  }

  class Repositorio {
    <<interface>>
    +guardar(Pedido p)*
    +todos() List
  }

  class PedidoEspecial

  Pedido <|-- PedidoEspecial
  Pedido "1" *-- "0..*" Linea : contiene
  Pedido ..> Repositorio : usa
"#;

#[test]
fn un_diagrama_de_clases_de_mermaid_llega_hasta_el_png() {
    use diagrammaker_lib::model::Rect;
    use diagrammaker_lib::model::shape_geometry::class_height;

    let imported = diagrammaker_lib::parse::mermaid::import(CLASES_MERMAID, 0);
    assert!(
        imported.ok(),
        "el Mermaid de ejemplo no deberia fallar: {:#?}",
        imported.diagnostics
    );

    let mut doc = imported.document;
    assert_eq!(doc.kind, DiagramKind::Class);

    // Colocar y medir es cosa de ELK y del navegador; aqui se hace a mano lo
    // justo para que llegue al pixel.
    let mut labels = HashMap::new();
    let mut x = 40.0;
    for index in 0..doc.nodes.len() {
        let (stereotype, attributes, operations) = match doc.nodes[index].class() {
            Some(box_) => (
                box_.has_stereotype(),
                box_.attributes.len(),
                box_.operations.len(),
            ),
            None => (false, 0, 0),
        };

        doc.nodes[index].geometry = Some(Rect::new(
            x,
            40.0,
            240.0,
            class_height(stereotype, attributes, operations),
        ));
        x += 280.0;

        labels.insert(
            doc.nodes[index].id.clone(),
            label(&doc.nodes[index].label, 13.0, LabelPlacement::Center),
        );
    }

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
        if let Some(text) = &edge.label {
            labels.insert(edge.id.clone(), label(text, 11.0, LabelPlacement::Center));
        }
    }

    let out = svg::render(&RenderInput {
        document: doc,
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
        measures: Default::default(),
    });

    for text in [
        "Pedido",
        "Repositorio",
        "PedidoEspecial",
        "- id: int",
        "+ creados: int",
        "+ confirmar()",
        "+ buscar(int id): Pedido",
        "+ guardar(Pedido p)",
        "interface",
        "contiene",
        "0..*",
    ] {
        assert!(out.contains(text), "falta `{text}` en el SVG");
    }

    // Lo estatico subrayado, lo abstracto en cursiva y el triangulo de la
    // herencia: los tres adornos que el dibujo tiene que distinguir.
    assert!(out.contains(r#"text-decoration="underline""#));
    assert!(out.contains(r#"font-style="italic""#));
    assert!(out.contains("dm-triangle-hollow"));

    check_png(&out, "clases-mermaid");
}

/// El mismo diagrama que `CLASES_MERMAID`, escrito en PlantUML.
const CLASES_PLANTUML: &str = r#"
@startuml
class Pedido {
  - id : int
  - total : Decimal
  + {static} creados : int
  --
  + confirmar()
  + buscar(id : int) : Pedido
}

interface Repositorio {
  + {abstract} guardar(p : Pedido)
  + todos() : List
}

class PedidoEspecial

Pedido <|-- PedidoEspecial
Pedido "1" *-- "0..*" Linea : contiene
Pedido ..> Repositorio : usa
@enduml
"#;

/// Resumen de un diagrama de clases, sin ids: lo que tiene que coincidir
/// entre idiomas.
///
/// Los parametros quedan fuera a proposito. El modelo los guarda **tal como
/// se escribieron**, y los dos idiomas los escriben distinto —`buscar(int
/// id)` en Mermaid, `buscar(id : int)` en PlantUML—, asi que no pueden
/// coincidir sin traducirlos, que es justo lo que se decidio no hacer.
fn resumen_clases(doc: &diagrammaker_lib::model::Document) -> Vec<String> {
    use diagrammaker_lib::model::ClassBox;

    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| {
            let empty = ClassBox::new();
            let box_ = node.class().unwrap_or(&empty);
            let sin_parametros = |m: &diagrammaker_lib::model::ClassMember, operation: bool| {
                let mut copia = m.clone();
                copia.parameters = String::new();
                copia.text(operation)
            };

            format!(
                "{} <<{}>> [{}] [{}]",
                node.label,
                box_.stereotype.clone().unwrap_or_default(),
                box_.attributes
                    .iter()
                    .map(|m| sin_parametros(m, false))
                    .collect::<Vec<_>>()
                    .join(", "),
                box_.operations
                    .iter()
                    .map(|m| sin_parametros(m, true))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
        .collect();

    for edge in &doc.edges {
        out.push(format!(
            "{}{} {:?} {}{}",
            doc.node(&edge.source.node).map_or("?", |n| &n.label),
            edge.source.label.clone().unwrap_or_default(),
            edge.kind,
            doc.node(&edge.target.node).map_or("?", |n| &n.label),
            edge.target.label.clone().unwrap_or_default(),
        ));
    }

    out
}

#[test]
fn el_mismo_diagrama_de_clases_en_mermaid_y_en_plantuml_da_lo_mismo() {
    // No que cada importador lea algo, sino que los dos lean lo mismo: es lo
    // unico que de verdad valida los dos a la vez.
    let de_mermaid = diagrammaker_lib::parse::mermaid::import(CLASES_MERMAID, 0);
    let de_plantuml = diagrammaker_lib::parse::plantuml::import(CLASES_PLANTUML, 0);

    assert!(de_mermaid.ok(), "{:#?}", de_mermaid.diagnostics);
    assert!(de_plantuml.ok(), "{:#?}", de_plantuml.diagnostics);

    let (a, b) = (de_mermaid.document, de_plantuml.document);

    assert_eq!(a.kind, b.kind);
    assert_eq!(resumen_clases(&a), resumen_clases(&b), "distinto contenido");

    // Y la parte que a proposito no coincide: cada uno guarda sus parametros
    // tal como venian escritos, sin traducirlos al otro idioma.
    let parametros = |doc: &diagrammaker_lib::model::Document| {
        doc.nodes
            .iter()
            .filter_map(|n| n.class())
            .flat_map(|box_| box_.operations.iter())
            .map(|m| m.parameters.clone())
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
    };

    assert_eq!(parametros(&a), ["int id", "Pedido p"]);
    assert_eq!(parametros(&b), ["id : int", "p : Pedido"]);
}

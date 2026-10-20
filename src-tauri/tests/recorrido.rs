//! El recorrido completo, de punta a punta.
//!
//! Las demas pruebas miran una pieza cada una: que el parser entiende algo,
//! que el generador lo vuelve a escribir, que el dibujo sale. Esta hace el
//! viaje entero que hace un usuario, en el mismo orden y por las mismas
//! funciones que llama la aplicacion:
//!
//!   pegar codigo → documento → validar → SVG → PNG y PDF → guardar `.dgm`
//!   → cerrar → reabrir → el mismo documento
//!
//! Lo que busca no es que cada pieza funcione —eso ya esta probado— sino que
//! **encajen**: que lo que sale de una entre en la siguiente sin que nadie
//! tenga que retocarlo por el camino.
//!
//! Lo unico que no pasa por aqui es el navegador. Medir el texto y rutear las
//! aristas es cosa suya, asi que esta prueba lo imita a lo bruto, igual que
//! `export_pipeline.rs`. Una prueba sobre el WebView de verdad necesitaria
//! `tauri-driver` y `msedgedriver`; como montarla esta en
//! `docs/pruebas-e2e.md`.

use std::collections::HashMap;

use diagrammaker_lib::model::{Document, Point, Rect, shape_geometry, validate};
use diagrammaker_lib::parse::{self, SourceLanguage};
use diagrammaker_lib::render::{
    EdgePath, ExportOptions, LabelLayout, LabelPlacement, RenderInput, RenderTheme, TextLine, pdf,
    raster, svg,
};
use diagrammaker_lib::{emit, io};

/// El esquema que escribiria alguien en el cuadro de importar.
const ESQUEMA: &str = r#"
Table clientes {
  id integer [pk, increment]
  nombre varchar(120) [not null]
  correo varchar(160) [unique]
}

Table pedidos {
  id integer [pk]
  cliente_id integer [not null]
  total decimal(10,2)
}

Ref: pedidos.cliente_id > clientes.id
"#;

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

/// Mide un texto a ojo. Es lo que en la aplicacion hace el navegador.
fn label(text: &str) -> LabelLayout {
    LabelLayout {
        lines: vec![TextLine {
            text: text.to_string(),
            width: text.chars().count() as f64 * 7.0,
        }],
        font_size: 13.0,
        line_height: 17.0,
        placement: LabelPlacement::Center,
        color: None,
        bold: false,
        italic: false,
    }
}

/// Coloca las tablas en fila, que es lo que haria ELK a lo bruto.
fn colocar(doc: &mut Document) {
    let mut x = 40.0;
    for node in &mut doc.nodes {
        let filas = node.er().map(|t| t.columns.len()).unwrap_or(0);
        node.geometry = Some(Rect::new(
            x,
            60.0,
            240.0,
            shape_geometry::table_height(filas),
        ));
        x += 330.0;
    }
}

/// Arma lo que la aplicacion manda a exportar: el documento ya colocado mas
/// las medidas que solo el lienzo conoce.
fn para_dibujar(doc: &Document) -> RenderInput {
    let mut labels = HashMap::new();
    let mut edges = HashMap::new();

    for node in &doc.nodes {
        labels.insert(node.id.clone(), label(&node.label));
    }

    for edge in &doc.edges {
        let a = doc
            .node(&edge.source.node)
            .and_then(|n| n.geometry)
            .unwrap();
        let b = doc
            .node(&edge.target.node)
            .and_then(|n| n.geometry)
            .unwrap();

        edges.insert(
            edge.id.clone(),
            EdgePath {
                d: format!(
                    "M {} {} L {} {}",
                    a.x + a.width,
                    a.center().y,
                    b.x,
                    b.center().y
                ),
                label_at: Some(Point::new((a.x + a.width + b.x) / 2.0, a.center().y)),
                hops: Vec::new(),
            },
        );
    }

    RenderInput {
        document: doc.clone(),
        labels,
        edges,
        theme: theme(),
        options: ExportOptions::default(),
        measures: Default::default(),
    }
}

/// Resumen del documento sin lo que cambia en cada importacion.
fn resumen(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|n| {
            let columnas: Vec<String> = n
                .er()
                .map(|t| {
                    t.columns
                        .iter()
                        .map(|c| format!("{} {} {}", c.data_type, c.name, c.badges().join("+")))
                        .collect()
                })
                .unwrap_or_default();
            format!("{} [{}]", n.label, columnas.join(", "))
        })
        .collect();
    out.sort();

    let mut aristas: Vec<String> = doc
        .edges
        .iter()
        .map(|e| {
            let nombre = |id: &str| doc.node(id).map(|n| n.label.clone()).unwrap_or_default();
            format!(
                "{} {} -> {} {}",
                e.source.label.clone().unwrap_or_default(),
                nombre(&e.source.node),
                nombre(&e.target.node),
                e.target.label.clone().unwrap_or_default()
            )
        })
        .collect();
    aristas.sort();
    out.extend(aristas);

    out
}

/// Carpeta temporal propia, que se borra al acabar.
struct Temporal(std::path::PathBuf);

impl Temporal {
    fn nueva(nombre: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("diagrammaker-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("no se pudo crear la carpeta temporal");
        Temporal(dir)
    }

    fn archivo(&self, nombre: &str) -> std::path::PathBuf {
        self.0.join(nombre)
    }
}

impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn de_pegar_codigo_a_reabrir_el_archivo() {
    let temp = Temporal::nueva("recorrido");

    /* 1. Pegar el codigo en el cuadro de importar. */
    let importado = parse::import(SourceLanguage::Dbml, ESQUEMA, 0);
    assert!(
        importado.ok(),
        "el esquema no deberia fallar: {:#?}",
        importado.diagnostics
    );

    let mut doc = importado.document;
    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.edges.len(), 1);

    // Llega sin geometria: colocarlo es cosa del auto-layout.
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));

    /* 2. El validador no tiene nada que decir de un esquema correcto. */
    assert!(
        validate::validate(&doc).diagnostics.is_empty(),
        "{:#?}",
        validate::validate(&doc).diagnostics
    );

    /* 3. Colocarlo y dibujarlo. */
    colocar(&mut doc);
    let dibujo = svg::render(&para_dibujar(&doc));

    for texto in ["clientes", "pedidos", "correo", "varchar(160)"] {
        assert!(dibujo.contains(texto), "falta `{texto}` en el SVG");
    }

    /* 4. Las tres salidas vienen del mismo SVG. */
    let png = raster::to_png(&dibujo, 2.0).expect("el PNG deberia salir");
    let documento_pdf = pdf::to_pdf(&dibujo).expect("el PDF deberia salir");

    assert!(
        png.starts_with(&[0x89, b'P', b'N', b'G']),
        "no parece un PNG"
    );
    assert!(documento_pdf.starts_with(b"%PDF"), "no parece un PDF");
    assert!(
        png.len() > 1_000 && documento_pdf.len() > 1_000,
        "salen vacios"
    );

    /* 5. Volver a generar el codigo y que vuelva a entrar igual. */
    let codigo = emit::emit(SourceLanguage::Dbml, &doc);
    let devuelta = parse::import(SourceLanguage::Dbml, &codigo, 0);

    assert!(devuelta.ok(), "{codigo}\n{:#?}", devuelta.diagnostics);
    assert_eq!(resumen(&doc), resumen(&devuelta.document), "{codigo}");

    /* 6. Guardar, cerrar y reabrir. */
    let ruta = temp.archivo("tienda.dgm");
    io::save(&ruta, &doc).expect("deberia guardar");
    assert!(ruta.exists(), "no se escribio el archivo");

    let reabierto = io::load(&ruta).expect("deberia abrir");

    // El documento vuelve entero, geometria incluida: lo que se coloco a mano
    // no se pierde al cerrar.
    assert_eq!(reabierto, doc, "el archivo no devuelve el mismo documento");
    assert!(reabierto.nodes.iter().all(|n| n.geometry.is_some()));
}

#[test]
fn el_recorrido_vale_para_los_tres_idiomas() {
    // El mismo viaje, mas corto, entrando por cada puerta. Lo que se mira es
    // que ninguna rama se quede sin colocar, sin dibujar o sin guardar.
    let temp = Temporal::nueva("idiomas");

    let casos: &[(&str, SourceLanguage, &str)] = &[
        (
            "mermaid",
            SourceLanguage::Mermaid,
            "classDiagram\n  class Pedido {\n    +int numero\n  }\n  class Linea\n  Pedido *-- Linea",
        ),
        (
            "plantuml",
            SourceLanguage::PlantUml,
            "@startuml\nstart\n:Recibir;\n:Cerrar;\nstop\n@enduml",
        ),
        (
            "dbml",
            SourceLanguage::Dbml,
            "Table t {\n  id integer [pk]\n}",
        ),
    ];

    for (nombre, language, source) in casos {
        let importado = parse::import(*language, source, 0);
        assert!(importado.ok(), "{nombre}: {:#?}", importado.diagnostics);

        let mut doc = importado.document;
        assert!(
            validate::validate(&doc).diagnostics.is_empty(),
            "{nombre} se queja: {:#?}",
            validate::validate(&doc).diagnostics
        );

        for (indice, node) in doc.nodes.iter_mut().enumerate() {
            node.geometry = Some(Rect::new(40.0 + indice as f64 * 260.0, 60.0, 200.0, 90.0));
        }

        let dibujo = svg::render(&para_dibujar(&doc));
        assert!(dibujo.contains("<svg"), "{nombre}: no salio SVG");
        assert!(
            raster::to_png(&dibujo, 1.0).is_ok(),
            "{nombre}: no salio PNG"
        );

        let ruta = temp.archivo(&format!("{nombre}.dgm"));
        io::save(&ruta, &doc).expect("deberia guardar");
        assert_eq!(io::load(&ruta).expect("deberia abrir"), doc, "{nombre}");
    }
}

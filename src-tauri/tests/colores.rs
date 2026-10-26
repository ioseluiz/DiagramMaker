//! Los colores que se ven en pantalla son los que salen en el archivo.
//!
//! El lienzo pinta con el motor CSS del navegador y la exportacion con
//! `resvg`, que no son el mismo analizador. Un color que el navegador
//! entiende y `resvg` no, `resvg` no lo rechaza: lo pinta **negro**. Asi
//! aparecio el fallo —el diagrama bien en pantalla, el PNG y el PDF en
//! negro— y asi puede volver a aparecer con cualquier color nuevo.
//!
//! Por eso aqui se mide el pixel en vez de comprobar que el SVG contenga el
//! texto del color: lo contiene igual cuando sale mal.

use std::collections::HashMap;

use diagrammaker_lib::model::{DiagramKind, Document, Edge, EdgeKind, Node, NodeShape};
use diagrammaker_lib::render::{
    ExportOptions, LabelLayout, LabelPlacement, RenderInput, RenderTheme, TextLine, raster, svg,
};

/* --- Medir el pixel ------------------------------------------------------- */

/// Color de un pixel del PNG, situado en fracciones de la imagen.
///
/// Se decodifica con el `tiny_skia` que ya trae `resvg`, igual que en
/// `export_pipeline.rs`: no hace falta otra dependencia solo para esto.
///
/// Ojo con el centro: ahi es donde va la etiqueta del nodo, asi que mide el
/// color del texto y no el del relleno. Por eso esto admite un punto.
fn pixel_en(png: &[u8], fx: f64, fy: f64) -> (u8, u8, u8, u8) {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).expect("PNG ilegible");
    let (ancho, alto) = (pixmap.width() as f64, pixmap.height() as f64);
    let datos = pixmap.data();

    let x = (ancho * fx) as u32;
    let y = (alto * fy) as u32;
    let i = ((y * pixmap.width() + x) * 4) as usize;
    (datos[i], datos[i + 1], datos[i + 2], datos[i + 3])
}

/// `true` si el pixel es negro o casi.
fn es_negro((r, g, b, _): (u8, u8, u8, u8)) -> bool {
    r < 40 && g < 40 && b < 40
}

/* --- Lo que `resvg` entiende por su cuenta -------------------------------- */

/// Un cuadrado del color pedido sobre fondo blanco, rasterizado sin pasar por
/// el generador: mide lo que entiende `resvg`, no lo que arregla la
/// aplicacion.
fn crudo(fill: &str) -> (u8, u8, u8, u8) {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
<rect x="0" y="0" width="40" height="40" fill="#ffffff"/>
<rect x="5" y="5" width="30" height="30" fill="{fill}"/>
</svg>"##
    );
    pixel_en(
        &raster::to_png(&svg, 1.0).expect("deberia rasterizar"),
        0.5,
        0.5,
    )
}

#[test]
fn resvg_entiende_la_sintaxis_clasica() {
    for color in [
        "#2563eb",
        "#2563eb29",
        "rgb(37,99,235)",
        "rgb(37, 99, 235)",
        "rgba(37, 99, 235, 0.16)",
        "hsl(210, 70%, 50%)",
    ] {
        let p = crudo(color);
        assert!(!es_negro(p), "`{color}` salio negro: {p:?}");
    }
}

#[test]
fn resvg_no_entiende_el_alfa_tras_la_barra_y_lo_pinta_negro() {
    // Este es el fallo, aislado, y es mas estrecho de lo que parecia:
    // `rgb(37 99 235)` con espacios si lo entiende. Lo que no traga es el
    // alfa detras de la barra, que es justo lo que escribia la paleta.
    //
    // Mientras esta prueba pase hay que seguir traduciendo en
    // `render::color`; el dia que `resvg` aprenda la sintaxis fallara, y esa
    // sera la senal de que la traduccion ya sobra.
    for color in ["rgb(37 99 235 / 0.16)", "rgb(0 0 0 / 16%)"] {
        assert!(
            es_negro(crudo(color)),
            "`{color}` ya se entiende: revisar si `render::color` hace falta"
        );
    }
}

/* --- Y lo que sale de la aplicacion --------------------------------------- */

/// Un diagrama de un solo nodo que llena el lienzo, con los colores dados.
///
/// Un solo nodo y sin margen para que el lienzo sea exactamente el nodo: asi
/// cualquier punto que no caiga sobre la etiqueta mide su relleno.
fn diagrama(fill: &str, stroke: &str, texto: &str) -> RenderInput {
    let mut doc = Document::new("colores", DiagramKind::Architecture, 0);

    let mut nodo = Node::new("n", "Nodo", NodeShape::Rectangle).at(0.0, 0.0, 200.0, 120.0);
    nodo.style.fill = Some(fill.into());
    nodo.style.stroke = Some(stroke.into());
    nodo.style.text_color = Some(texto.into());
    doc.nodes.push(nodo);

    let mut labels = HashMap::new();
    labels.insert(
        "n".to_string(),
        LabelLayout {
            lines: vec![TextLine {
                text: "Nodo".into(),
                width: 30.0,
            }],
            font_size: 13.0,
            line_height: 17.0,
            placement: LabelPlacement::Center,
            color: None,
            bold: false,
            italic: false,
        },
    );

    RenderInput {
        document: doc,
        labels,
        edges: HashMap::new(),
        theme: RenderTheme {
            background: "#ffffff".into(),
            node_fill: "#ffffff".into(),
            node_stroke: "#b6bec9".into(),
            node_text: "#16191d".into(),
            edge_stroke: "#b6bec9".into(),
            edge_text: "#5b6471".into(),
            font_family: "Segoe UI, system-ui, sans-serif".into(),
        },
        options: ExportOptions {
            padding: 0.0,
            opaque_background: true,
            ..Default::default()
        },
        measures: Default::default(),
    }
}

/// Exporta el diagrama y devuelve el color de su relleno.
fn exportado(fill: &str) -> (u8, u8, u8, u8) {
    let input = diagrama(fill, "#2563eb", "#16191d");
    let svg = svg::render(&input);

    // A un quinto de la altura: dentro del relleno y por encima del texto.
    pixel_en(
        &raster::to_png(&svg, 1.0).expect("deberia rasterizar"),
        0.5,
        0.2,
    )
}

#[test]
fn los_tintes_de_la_paleta_salen_con_su_color() {
    // Los seis de `src/model/palette.ts`, en la sintaxis moderna con la que
    // quedaron guardados los documentos de antes del arreglo. Tienen que
    // salir igual que los nuevos.
    let tintes = [
        ("azul", "rgb(37 99 235 / 0.16)"),
        ("verde", "rgb(21 128 61 / 0.16)"),
        ("ambar", "rgb(217 119 6 / 0.16)"),
        ("rojo", "rgb(194 52 29 / 0.16)"),
        ("morado", "rgb(124 58 237 / 0.16)"),
        ("turquesa", "rgb(13 148 136 / 0.16)"),
    ];

    for (nombre, color) in tintes {
        let p = exportado(color);
        assert!(!es_negro(p), "el tinte {nombre} salio negro: {p:?}");
    }
}

#[test]
fn un_tinte_translucido_queda_palido_sobre_el_fondo() {
    // No basta con que no sea negro: con un 16 % de alfa sobre blanco tiene
    // que quedar muy claro. Si saliera opaco, el texto de encima no se leeria
    // —y un alfa ignorado es otra forma de que la traduccion este mal.
    let (r, g, b, _) = exportado("rgb(37 99 235 / 0.16)");

    assert!(
        r > 190 && g > 190 && b > 230,
        "deberia ser un azul palido y es ({r}, {g}, {b})"
    );
}

#[test]
fn la_forma_nueva_y_la_vieja_dan_el_mismo_color() {
    // La paleta ya escribe `rgba(...)`, pero los `.dgm` guardados llevan
    // dentro la forma antigua. Las dos tienen que acabar en el mismo pixel.
    assert_eq!(
        exportado("rgb(37 99 235 / 0.16)"),
        exportado("rgba(37, 99, 235, 0.16)")
    );
}

#[test]
fn ningun_color_moderno_llega_al_svg_por_ningun_campo() {
    // La red de seguridad: el relleno, el trazo, el texto, el tema, la
    // etiqueta y la arista a la vez. Si manana alguien anade un sitio que
    // escriba un color, esta prueba lo ve sin que haya que acordarse de
    // ampliarla campo por campo.
    let moderno = "rgb(37 99 235 / 0.16)";

    let mut input = diagrama(moderno, moderno, moderno);

    input.theme = RenderTheme {
        background: moderno.into(),
        node_fill: moderno.into(),
        node_stroke: moderno.into(),
        node_text: moderno.into(),
        edge_stroke: moderno.into(),
        edge_text: moderno.into(),
        font_family: "Segoe UI, system-ui, sans-serif".into(),
    };

    if let Some(label) = input.labels.get_mut("n") {
        label.color = Some(moderno.into());
    }

    let mut otro = Node::new("m", "Otro", NodeShape::Rectangle).at(260.0, 0.0, 200.0, 120.0);
    otro.style.fill = Some(moderno.into());
    input.document.nodes.push(otro);

    let mut arista = Edge::new("e", "n", "m", EdgeKind::Directed);
    arista.style.stroke = Some(moderno.into());
    arista.style.text_color = Some(moderno.into());
    input.document.edges.push(arista);

    let svg = svg::render(&input);

    // Un `rgb(` cuyo contenido no lleve comas es la sintaxis moderna.
    for (i, _) in svg.match_indices("rgb(") {
        let resto = &svg[i + 4..];
        let fin = resto.find(')').unwrap_or(resto.len());
        assert!(
            resto[..fin].contains(','),
            "queda un color sin traducir: `rgb({})`",
            &resto[..fin]
        );
    }

    // Y que siga siendo un SVG valido: uno que no se analiza tampoco tiene
    // colores malos, y pasaria el barrido de arriba sin merecerlo. Que el
    // color salga bien lo miden las pruebas de pixel anteriores.
    raster::to_png(&svg, 1.0).expect("el SVG traducido deberia rasterizar");
}

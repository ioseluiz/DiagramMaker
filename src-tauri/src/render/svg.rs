//! Generador de SVG.
//!
//! Emite SVG 1.1 autocontenido: sin variables CSS, sin clases externas y sin
//! fuentes incrustadas. Lo que sale tiene que abrirse igual en un navegador,
//! en Inkscape y en el visor de imagenes de Windows.

use std::fmt::Write as _;

use crate::model::edge::EdgeKind;
use crate::model::shape_geometry::{
    Primitive, TABLE_HEADER_FONT_SIZE, TABLE_HEADER_HEIGHT, TABLE_PADDING, TABLE_ROW_FONT_SIZE,
    primitives, table_primitives, table_row_rect,
};
use crate::model::{Document, Edge, ErTable, Node, Rect};

use super::{EdgePath, HOP_RADIUS, Hop, LabelLayout, LabelPlacement, RenderInput, RenderTheme};

/// Alto de la solapa del paquete; debe coincidir con `shape_geometry`.
const TAB_HEIGHT: f64 = 22.0;
/// Separacion entre la figura del actor y su nombre.
const BELOW_GAP: f64 = 4.0;

/// Area que ocupa el diagrama, ya con el margen de exportacion.
pub fn viewport(input: &RenderInput) -> Rect {
    let content = bounds(&input.document, input);
    content.inflate(input.options.padding)
}

/// Caja que envuelve todo lo dibujable. Tiene en cuenta las etiquetas que
/// sobresalen de su nodo (el nombre de un actor cuelga por debajo) y los
/// trazados de arista, que pueden salirse si llevan puntos intermedios.
fn bounds(doc: &Document, input: &RenderInput) -> Rect {
    let mut result: Option<Rect> = None;
    let mut add = |r: Rect| {
        result = Some(match result {
            Some(acc) => acc.union(&r),
            None => r,
        });
    };

    for node in &doc.nodes {
        let Some(g) = node.geometry else { continue };
        add(g);

        if let Some(label) = input.labels.get(&node.id)
            && label.placement == LabelPlacement::Below
            && !label.is_empty()
        {
            let width = label.width().max(g.width);
            add(Rect::new(
                g.center().x - width / 2.0,
                g.bottom() + BELOW_GAP,
                width,
                label.height(),
            ));
        }
    }

    for edge in &doc.edges {
        if let Some(path) = input.edges.get(&edge.id)
            && let Some(r) = path_bounds(&path.d)
        {
            add(r);
        }
    }

    result.unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0))
}

/// Caja envolvente aproximada de un trazado: los puntos de control de las
/// curvas se tratan como si estuvieran sobre la linea. Sobreestima un poco,
/// que para calcular un margen es el lado correcto en el que equivocarse.
fn path_bounds(d: &str) -> Option<Rect> {
    let numbers: Vec<f64> = d
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();

    if numbers.len() < 2 {
        return None;
    }

    let xs: Vec<f64> = numbers.iter().step_by(2).copied().collect();
    let ys: Vec<f64> = numbers.iter().skip(1).step_by(2).copied().collect();

    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    if !min_x.is_finite() || !min_y.is_finite() {
        return None;
    }

    Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

/// Escapa los caracteres que no pueden aparecer tal cual en texto XML.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Formatea un numero sin ceros de cola: `30` en vez de `30.0000`.
fn n(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == rounded.trunc() && rounded.abs() < 1e15 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Genera el SVG completo.
pub fn render(input: &RenderInput) -> String {
    let view = viewport(input);
    let theme = &input.theme;

    let mut out = String::with_capacity(4096);

    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="{x} {y} {w} {h}">"#,
        w = n(view.width),
        h = n(view.height),
        x = n(view.x),
        y = n(view.y),
    );

    let title = input.document.meta.title.trim();
    if !title.is_empty() {
        let _ = write!(out, "<title>{}</title>", escape(title));
    }

    out.push_str(&markers(theme));

    if input.options.opaque_background {
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
            n(view.x),
            n(view.y),
            n(view.width),
            n(view.height),
            escape(&theme.background),
        );
    }

    // Orden de pintado, de abajo a arriba:
    //
    //   1. trazados de arista  -> las cajas los tapan donde se solapan
    //   2. nodos y sus etiquetas
    //   3. texto de las aristas -> nunca debe quedar debajo de una caja
    //
    // El tercer paso existe porque una cardinalidad se dibuja pegada al borde
    // del nodo: si fuera con el trazado, el relleno de la caja se la comeria.
    for edge in &input.document.edges {
        if let Some(path) = input.edges.get(&edge.id) {
            out.push_str(&render_edge_path(edge, path, &input.theme));
        }
    }

    // Los contenedores se dibujan antes que su contenido.
    let mut nodes: Vec<&Node> = input.document.nodes.iter().collect();
    nodes.sort_by_key(|n| depth_of(&input.document, n));
    for node in nodes {
        out.push_str(&render_node(node, input));
    }

    for edge in &input.document.edges {
        if let Some(path) = input.edges.get(&edge.id) {
            out.push_str(&render_edge_text(edge, path, input));
        }
    }

    out.push_str("</svg>");
    out
}

/// Cuantos contenedores tiene por encima un nodo.
fn depth_of(doc: &Document, node: &Node) -> usize {
    let mut depth = 0;
    let mut cursor = node.parent.as_deref();
    // El tope evita colgarse si el documento trae un ciclo que `validate`
    // habria senalado pero que nadie corrigio.
    while let Some(id) = cursor {
        depth += 1;
        if depth > doc.nodes.len() {
            break;
        }
        cursor = doc.node(id).and_then(|n| n.parent.as_deref());
    }
    depth
}

fn markers(theme: &RenderTheme) -> String {
    let stroke = escape(&theme.edge_stroke);
    let fill = escape(&theme.background);

    format!(
        concat!(
            "<defs>",
            r#"<marker id="dm-arrow-open" viewBox="0 0 12 12" markerWidth="12" markerHeight="12" refX="11" refY="6" orient="auto-start-reverse" markerUnits="userSpaceOnUse">"#,
            r#"<path d="M1,1 L11,6 L1,11" fill="none" stroke="{stroke}" stroke-width="1.5"/></marker>"#,
            r#"<marker id="dm-arrow-filled" viewBox="0 0 12 12" markerWidth="11" markerHeight="11" refX="10" refY="6" orient="auto-start-reverse" markerUnits="userSpaceOnUse">"#,
            r#"<path d="M1,1.5 L11,6 L1,10.5 z" fill="{stroke}"/></marker>"#,
            r#"<marker id="dm-triangle-hollow" viewBox="0 0 14 14" markerWidth="14" markerHeight="14" refX="13" refY="7" orient="auto-start-reverse" markerUnits="userSpaceOnUse">"#,
            r#"<path d="M1,1 L13,7 L1,13 z" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/></marker>"#,
            r#"<marker id="dm-diamond-filled" viewBox="0 0 18 12" markerWidth="18" markerHeight="12" refX="1" refY="6" orient="auto-start-reverse" markerUnits="userSpaceOnUse">"#,
            r#"<path d="M1,6 L9,1.5 L17,6 L9,10.5 z" fill="{stroke}"/></marker>"#,
            r#"<marker id="dm-diamond-hollow" viewBox="0 0 18 12" markerWidth="18" markerHeight="12" refX="1" refY="6" orient="auto-start-reverse" markerUnits="userSpaceOnUse">"#,
            r#"<path d="M1,6 L9,1.5 L17,6 L9,10.5 z" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/></marker>"#,
            "</defs>",
        ),
        stroke = stroke,
        fill = fill,
    )
}

/// Marcadores de cada tipo de relacion. Debe coincidir con el mapa de
/// `src/canvas/graph/adapter.ts`.
fn markers_for(kind: EdgeKind) -> (Option<&'static str>, Option<&'static str>) {
    match kind {
        EdgeKind::Association => (None, None),
        EdgeKind::Directed | EdgeKind::Dependency => (None, Some("dm-arrow-open")),
        EdgeKind::Inheritance | EdgeKind::Realization => (None, Some("dm-triangle-hollow")),
        EdgeKind::Composition => (Some("dm-diamond-filled"), None),
        EdgeKind::Aggregation => (Some("dm-diamond-hollow"), None),
        EdgeKind::Flow => (None, Some("dm-arrow-filled")),
    }
}

/// La linea, sus marcadores y los puentes de los cruces.
fn render_edge_path(edge: &Edge, path: &EdgePath, theme: &RenderTheme) -> String {
    let stroke = edge.style.stroke.as_deref().unwrap_or(&theme.edge_stroke);
    let width = edge.style.stroke_width.unwrap_or(1.5);
    let dashed = edge
        .style
        .dashed
        .unwrap_or_else(|| edge.kind.is_dashed_by_default());

    let (start, end) = markers_for(edge.kind);

    let mut out = String::new();
    let _ = write!(
        out,
        r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}""#,
        escape(&path.d),
        escape(stroke),
        n(width),
    );
    if dashed {
        out.push_str(r#" stroke-dasharray="6 4""#);
    }
    if let Some(m) = start {
        let _ = write!(out, r##" marker-start="url(#{m})""##);
    }
    if let Some(m) = end {
        let _ = write!(out, r##" marker-end="url(#{m})""##);
    }
    out.push_str("/>");

    for hop in &path.hops {
        out.push_str(&render_hop(hop, stroke, width, theme));
    }

    out
}

/// Puente sobre un cruce: borra el tramo de linea y dibuja el arco encima.
///
/// Es la misma construccion que usa el lienzo. Sin el trazo que borra, la
/// linea recta seguiria viendose por debajo del arco y el cruce no se
/// entenderia.
fn render_hop(hop: &Hop, stroke: &str, width: f64, theme: &RenderTheme) -> String {
    let dx = hop.angle.cos() * HOP_RADIUS;
    let dy = hop.angle.sin() * HOP_RADIUS;

    let (x0, y0) = (hop.x - dx, hop.y - dy);
    let (x1, y1) = (hop.x + dx, hop.y + dy);

    format!(
        concat!(
            r#"<path d="M {x0} {y0} L {x1} {y1}" fill="none" stroke="{bg}" stroke-width="{mask}" stroke-linecap="butt"/>"#,
            r#"<path d="M {x0} {y0} A {r} {r} 0 0 1 {x1} {y1}" fill="none" stroke="{stroke}" stroke-width="{w}" stroke-linecap="round"/>"#,
        ),
        x0 = n(x0),
        y0 = n(y0),
        x1 = n(x1),
        y1 = n(y1),
        r = n(HOP_RADIUS),
        bg = escape(&theme.background),
        mask = n(width + 1.5),
        stroke = escape(stroke),
        w = n(width),
    )
}

/// Etiqueta central y cardinalidades. Se dibujan despues de los nodos.
fn render_edge_text(edge: &Edge, path: &EdgePath, input: &RenderInput) -> String {
    let theme = &input.theme;
    let mut out = String::new();

    // Etiqueta central, sobre un recuadro del color del fondo para que la
    // linea no la atraviese.
    if let (Some(label), Some(at)) = (input.labels.get(&edge.id), path.label_at)
        && !label.is_empty()
    {
        let w = label.width();
        let h = label.height();
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
            n(at.x - w / 2.0 - 3.0),
            n(at.y - h / 2.0),
            n(w + 6.0),
            n(h),
            escape(&theme.background),
        );
        out.push_str(&text_block(
            label,
            at.x,
            at.y - h / 2.0,
            label.color.as_deref().unwrap_or(&theme.edge_text),
            theme,
        ));
    }

    // Cardinalidades de los extremos: solo tienen sentido con el trazado, que
    // es quien sabe donde empieza y acaba la linea.
    out.push_str(&endpoint_labels(edge, path, input));

    out
}

/// Cardinalidades pegadas a cada extremo de la arista.
fn endpoint_labels(edge: &Edge, path: &EdgePath, input: &RenderInput) -> String {
    let Some((first, last)) = path_ends(&path.d) else {
        return String::new();
    };

    let theme = &input.theme;
    let size = input
        .labels
        .get(&edge.id)
        .map(|l| l.font_size)
        .unwrap_or(11.0);

    let mut out = String::new();
    for (point, text) in [
        (first, edge.source.label.as_deref()),
        (last, edge.target.label.as_deref()),
    ] {
        let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}" text-anchor="middle">{}</text>"#,
            n(point.0),
            n(point.1 - 6.0),
            escape(&theme.font_family),
            n(size),
            escape(&theme.edge_text),
            escape(text),
        );
    }
    out
}

/// Primer y ultimo punto de un trazado.
fn path_ends(d: &str) -> Option<((f64, f64), (f64, f64))> {
    let numbers: Vec<f64> = d
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();

    if numbers.len() < 4 {
        return None;
    }

    Some((
        (numbers[0], numbers[1]),
        (numbers[numbers.len() - 2], numbers[numbers.len() - 1]),
    ))
}

fn render_node(node: &Node, input: &RenderInput) -> String {
    let Some(g) = node.geometry else {
        return String::new();
    };

    let theme = &input.theme;
    let fill = node.style.fill.as_deref().unwrap_or(&theme.node_fill);
    let stroke = node.style.stroke.as_deref().unwrap_or(&theme.node_stroke);
    let width = node.style.stroke_width.unwrap_or(1.5);
    let dashed = node.style.dashed.unwrap_or(false);

    let mut out = String::new();
    let _ = write!(out, r#"<g transform="translate({} {})">"#, n(g.x), n(g.y));

    // El contorno de una tabla depende de cuantas columnas tenga, no solo de
    // su caja, asi que sale de `table_primitives`.
    let outline = match node.er() {
        Some(table) => table_primitives(table.columns.len(), g.width, g.height),
        None => primitives(node.shape, g.width, g.height),
    };

    for primitive in outline {
        let (body_fill, body_stroke) = if primitive.is_filled() {
            (fill, stroke)
        } else {
            ("none", stroke)
        };

        let mut attrs = format!(
            r#" fill="{}" stroke="{}" stroke-width="{}""#,
            escape(body_fill),
            escape(body_stroke),
            n(width)
        );
        if dashed {
            attrs.push_str(r#" stroke-dasharray="6 4""#);
        }

        match primitive {
            Primitive::Rect {
                x,
                y,
                width: w,
                height: h,
                radius,
            } => {
                let _ = write!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}""#,
                    n(x),
                    n(y),
                    n(w),
                    n(h)
                );
                if let Some(r) = radius {
                    let _ = write!(out, r#" rx="{}" ry="{}""#, n(r), n(r));
                }
                let _ = write!(out, "{attrs}/>");
            }
            Primitive::Ellipse { cx, cy, rx, ry } => {
                let _ = write!(
                    out,
                    r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}"{attrs}/>"#,
                    n(cx),
                    n(cy),
                    n(rx),
                    n(ry)
                );
            }
            Primitive::Circle { cx, cy, r } => {
                let _ = write!(
                    out,
                    r#"<circle cx="{}" cy="{}" r="{}"{attrs}/>"#,
                    n(cx),
                    n(cy),
                    n(r)
                );
            }
            Primitive::Path { d } | Primitive::Line { d } => {
                let _ = write!(
                    out,
                    r#"<path d="{}"{attrs} stroke-linecap="round"/>"#,
                    escape(&d)
                );
            }
        }
    }

    out.push_str("</g>");

    // Las tablas rotulan sus propias filas: el texto va alineado a los bordes
    // y no centrado, asi que no hace falta medirlo en el navegador.
    if let Some(table) = node.er() {
        out.push_str(&render_table_text(node, table, g, input));
        return out;
    }

    if let Some(label) = input.labels.get(&node.id)
        && !label.is_empty()
    {
        let color = label
            .color
            .as_deref()
            .or(node.style.text_color.as_deref())
            .unwrap_or(&theme.node_text);

        let (x, top) = match label.placement {
            LabelPlacement::Center => (g.center().x, g.center().y - label.height() / 2.0),
            LabelPlacement::Below => (g.center().x, g.bottom() + BELOW_GAP),
            // La solapa se rotula alineada a la izquierda.
            LabelPlacement::Tab => (g.x + 8.0 + label.width() / 2.0, g.y + 3.0),
        };

        out.push_str(&text_block(label, x, top, color, theme));
    }

    out
}

/// Nombre de la tabla y el texto de cada columna.
///
/// Se alinea a los bordes, no al centro, asi que basta con el margen fijo y
/// no hacen falta anchos medidos: lo unico que se necesita es el texto, que ya
/// esta en el documento.
fn render_table_text(node: &Node, table: &ErTable, g: Rect, input: &RenderInput) -> String {
    let theme = &input.theme;
    let color = node.style.text_color.as_deref().unwrap_or(&theme.node_text);

    let mut out = String::new();

    // Cabecera.
    let header_baseline = g.y + TABLE_HEADER_HEIGHT / 2.0 + TABLE_HEADER_FONT_SIZE * 0.35;
    let _ = write!(
        out,
        r#"<text x="{}" y="{}" font-family="{}" font-size="{}" font-weight="600" fill="{}">{}</text>"#,
        n(g.x + TABLE_PADDING),
        n(header_baseline),
        escape(&theme.font_family),
        n(TABLE_HEADER_FONT_SIZE),
        escape(color),
        escape(&node.label),
    );

    for (index, column) in table.columns.iter().enumerate() {
        let row = table_row_rect(index, g.width);
        let baseline = g.y + row.center().y + TABLE_ROW_FONT_SIZE * 0.35;

        // Nombre a la izquierda; las claves primarias en negrita para que se
        // encuentren de un vistazo.
        let weight = if column.primary_key {
            r#" font-weight="600""#
        } else {
            ""
        };
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}"{weight}>{}</text>"#,
            n(g.x + TABLE_PADDING),
            n(baseline),
            escape(&theme.font_family),
            n(TABLE_ROW_FONT_SIZE),
            escape(color),
            escape(&column.name),
        );

        // Tipo y marcas a la derecha, en tono atenuado.
        let mut right = column.data_type.clone();
        let badges = column.badges();
        if !badges.is_empty() {
            if !right.is_empty() {
                right.push(' ');
            }
            right.push_str(&badges.join(" "));
        }

        if !right.is_empty() {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}" text-anchor="end">{}</text>"#,
                n(g.right() - TABLE_PADDING),
                n(baseline),
                escape(&theme.font_family),
                n(TABLE_ROW_FONT_SIZE),
                escape(&theme.edge_text),
                escape(&right),
            );
        }
    }

    out
}

/// Bloque de texto centrado horizontalmente en `x`, con la primera linea
/// empezando en `top`.
///
/// `dominant-baseline` no es fiable entre visores, asi que la linea base se
/// calcula a mano: `0.8 * font_size` por debajo del alto de linea centrado es
/// una aproximacion estable de la altura de las mayusculas.
fn text_block(label: &LabelLayout, x: f64, top: f64, color: &str, theme: &RenderTheme) -> String {
    let mut out = String::new();

    let _ = write!(
        out,
        r#"<text font-family="{}" font-size="{}" fill="{}" text-anchor="middle""#,
        escape(&theme.font_family),
        n(label.font_size),
        escape(color),
    );
    if label.bold {
        out.push_str(r#" font-weight="600""#);
    }
    if label.italic {
        out.push_str(r#" font-style="italic""#);
    }
    out.push('>');

    for (index, line) in label.lines.iter().enumerate() {
        let center = top + label.line_height * (index as f64 + 0.5);
        let baseline = center + label.font_size * 0.35;
        let _ = write!(
            out,
            r#"<tspan x="{}" y="{}">{}</tspan>"#,
            n(x),
            n(baseline),
            escape(&line.text)
        );
    }

    out.push_str("</text>");
    out
}

/// Posicion vertical de la solapa del paquete, para quien necesite alinearse
/// con ella.
pub const PACKAGE_TAB_HEIGHT: f64 = TAB_HEIGHT;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramKind, Document, Node, NodeShape};
    use crate::render::fixtures;

    #[test]
    fn el_svg_lleva_cabecera_y_viewbox() {
        let svg = render(&fixtures::input());

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("viewBox="));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn el_viewbox_envuelve_el_contenido_con_margen() {
        let input = fixtures::input();
        let view = viewport(&input);

        // Contenido: x de 0 a 420, y de 0 a 90. Margen por defecto 24.
        assert_eq!(view.x, -24.0);
        assert_eq!(view.y, -24.0);
        assert_eq!(view.width, 420.0 + 48.0);
        assert_eq!(view.height, 90.0 + 48.0);
    }

    #[test]
    fn el_fondo_opaco_se_puede_quitar() {
        let mut input = fixtures::input();

        assert!(render(&input).contains(r##"fill="#ffffff""##));

        input.options.opaque_background = false;
        let transparent = render(&input);
        // Sin fondo no hay rectangulo que cubra todo el viewBox.
        assert!(!transparent.contains(r#"<rect x="-24" y="-24""#));
    }

    #[test]
    fn no_quedan_variables_css_en_la_salida() {
        let svg = render(&fixtures::input());

        assert!(
            !svg.contains("var(--"),
            "el SVG exportado no puede depender del tema de la app"
        );
    }

    #[test]
    fn las_etiquetas_se_escapan() {
        let mut input = fixtures::input();
        input
            .labels
            .insert("a".into(), fixtures::label("A & <B>", 40.0));

        let svg = render(&input);

        assert!(svg.contains("A &amp; &lt;B&gt;"));
        assert!(!svg.contains("<B>"));
    }

    #[test]
    fn el_titulo_del_documento_va_en_el_svg() {
        let mut input = fixtures::input();
        input.document.meta.title = "Mi diagrama".into();

        assert!(render(&input).contains("<title>Mi diagrama</title>"));
    }

    #[test]
    fn un_flujo_lleva_flecha_rellena_al_final() {
        let svg = render(&fixtures::input());

        assert!(svg.contains(r##"marker-end="url(#dm-arrow-filled)""##));
    }

    #[test]
    fn una_herencia_lleva_triangulo_hueco() {
        let mut input = fixtures::input();
        input.document.edges[0].kind = EdgeKind::Inheritance;

        assert!(render(&input).contains(r##"marker-end="url(#dm-triangle-hollow)""##));
    }

    #[test]
    fn un_cruce_dibuja_su_puente() {
        let mut input = fixtures::input();
        input.edges.get_mut("e1").unwrap().hops = vec![Hop {
            x: 230.0,
            y: 37.0,
            angle: 0.0,
        }];

        let svg = render(&input);

        // El arco, y el trazo que borra la linea por debajo.
        assert!(
            svg.contains(&format!("A {} {} 0 0 1", n(HOP_RADIUS), n(HOP_RADIUS))),
            "{svg}"
        );
        assert!(svg.contains(r#"stroke-linecap="butt""#), "{svg}");
    }

    #[test]
    fn sin_cruces_no_hay_puentes() {
        let svg = render(&fixtures::input());

        assert!(!svg.contains(r#"stroke-linecap="butt""#));
    }

    #[test]
    fn el_relleno_y_el_color_de_texto_de_un_nodo_llegan_al_svg() {
        // Es lo que hace posible un estilo de relleno solido, como los
        // niveles del modelo C4: sin el color de texto propio, el del tema
        // quedaria ilegible encima.
        let mut input = fixtures::input();
        input.document.nodes[0].style.fill = Some("#08427B".into());
        input.document.nodes[0].style.stroke = Some("#073B6F".into());
        input.document.nodes[0].style.text_color = Some("#FFFFFF".into());

        let svg = render(&input);

        assert!(svg.contains(r##"fill="#08427B""##), "{svg}");
        assert!(svg.contains(r##"stroke="#073B6F""##), "{svg}");
        assert!(svg.contains(r##"fill="#FFFFFF""##), "{svg}");
    }

    #[test]
    fn una_dependencia_sale_punteada() {
        let mut input = fixtures::input();
        input.document.edges[0].kind = EdgeKind::Dependency;

        assert!(render(&input).contains(r#"stroke-dasharray="6 4""#));
    }

    #[test]
    fn las_aristas_se_dibujan_antes_que_los_nodos() {
        let svg = render(&fixtures::input());

        let edge = svg.find("M 160 30").expect("falta la arista");
        let node = svg.find("<g transform=").expect("falta el nodo");

        assert!(edge < node, "una arista por encima de las cajas se ve mal");
    }

    #[test]
    fn el_texto_de_las_aristas_va_por_encima_de_los_nodos() {
        // Una cardinalidad se dibuja pegada al borde del nodo. Si saliera con
        // el trazado, el relleno de la caja la taparia.
        let mut input = fixtures::input();
        input.document.edges[0].target.label = Some("0..*".into());

        let svg = render(&input);

        let ultimo_nodo = svg.rfind("<g transform=").expect("falta algun nodo");
        let cardinalidad = svg.find(">0..*</text>").expect("falta la cardinalidad");

        assert!(cardinalidad > ultimo_nodo);
    }

    #[test]
    fn la_etiqueta_central_tambien_va_por_encima() {
        let svg = render(&fixtures::input());

        let ultimo_nodo = svg.rfind("<g transform=").expect("falta algun nodo");
        let etiqueta = svg.find("consulta").expect("falta la etiqueta");

        assert!(etiqueta > ultimo_nodo);
    }

    #[test]
    fn los_contenedores_se_dibujan_antes_que_su_contenido() {
        let mut doc = Document::new("d", DiagramKind::Architecture, 0);
        doc.nodes.push(
            Node::new("hijo", "Hijo", NodeShape::Rectangle)
                .at(20.0, 40.0, 100.0, 40.0)
                .with_parent("pkg"),
        );
        doc.nodes
            .push(Node::new("pkg", "Paquete", NodeShape::Package).at(0.0, 0.0, 220.0, 160.0));

        let input = RenderInput {
            document: doc,
            labels: Default::default(),
            edges: Default::default(),
            theme: fixtures::theme(),
            options: Default::default(),
        };

        let svg = render(&input);
        let padre = svg.find("translate(0 0)").expect("falta el paquete");
        let hijo = svg.find("translate(20 40)").expect("falta el hijo");

        assert!(padre < hijo);
    }

    #[test]
    fn el_nombre_del_actor_cuelga_por_debajo_y_entra_en_el_viewbox() {
        let mut doc = Document::new("d", DiagramKind::UseCase, 0);
        doc.nodes
            .push(Node::new("a", "Operador", NodeShape::Actor).at(0.0, 0.0, 40.0, 80.0));

        let mut label = fixtures::label("Operador", 120.0);
        label.placement = LabelPlacement::Below;

        let input = RenderInput {
            document: doc,
            labels: std::collections::HashMap::from([("a".to_string(), label)]),
            edges: Default::default(),
            theme: fixtures::theme(),
            options: Default::default(),
        };

        let view = viewport(&input);

        // La etiqueta mide 120 y el actor 40: el viewBox tiene que ensancharse.
        assert!(
            view.width >= 120.0 + 48.0,
            "viewBox demasiado estrecho: {view:?}"
        );
        assert!(view.height > 80.0 + 48.0, "la etiqueta de abajo no cabe");
    }

    #[test]
    fn un_nodo_sin_geometria_no_rompe_la_exportacion() {
        let mut input = fixtures::input();
        input
            .document
            .nodes
            .push(Node::new("sin", "Sin sitio", NodeShape::Rectangle));

        let svg = render(&input);

        assert!(svg.ends_with("</svg>"));
        assert!(!svg.contains("Sin sitio"));
    }

    #[test]
    fn un_documento_vacio_produce_un_svg_valido() {
        let input = RenderInput {
            document: Document::new("d", DiagramKind::Architecture, 0),
            labels: Default::default(),
            edges: Default::default(),
            theme: fixtures::theme(),
            options: Default::default(),
        };

        let svg = render(&input);

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn las_cardinalidades_salen_en_los_extremos() {
        let mut input = fixtures::input();
        input.document.edges[0].source.label = Some("1".into());
        input.document.edges[0].target.label = Some("0..*".into());

        let svg = render(&input);

        assert!(svg.contains(">1</text>"), "{svg}");
        assert!(svg.contains(">0..*</text>"), "{svg}");
    }

    #[test]
    fn los_numeros_salen_sin_ceros_de_cola() {
        assert_eq!(n(30.0), "30");
        assert_eq!(n(37.5), "37.5");
        assert_eq!(n(0.750001), "0.75");
        assert_eq!(n(-24.0), "-24");
    }

    #[test]
    fn path_bounds_encuentra_los_extremos() {
        let r = path_bounds("M 10 20 L 100 5 L 50 80").unwrap();

        assert_eq!(r, Rect::new(10.0, 5.0, 90.0, 75.0));
    }

    #[test]
    fn path_ends_devuelve_primer_y_ultimo_punto() {
        let (first, last) = path_ends("M 10 20 L 100 5 L 50 80").unwrap();

        assert_eq!(first, (10.0, 20.0));
        assert_eq!(last, (50.0, 80.0));
    }
}

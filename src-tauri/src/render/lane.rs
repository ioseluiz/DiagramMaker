//! Dibujo de un diagrama de secuencia.
//!
//! El reparto ya viene hecho de `model::lane_geometry`; aqui solo se traduce
//! a SVG. El orden de pintado importa, igual que en el generador de grafo:
//! primero los marcos, que son el fondo, y encima lo que ocurre dentro.
//!
//! Los textos llegan medidos desde el navegador en `input.labels`, indexados
//! por el id de la pieza a la que pertenecen: el nodo para la cabecera de un
//! participante, el paso para el texto de un mensaje o una nota, el bloque
//! para el rotulo del marco y la rama para el de un separador.
//!
//! Limitacion conocida: la cabecera de un participante se dibuja siempre como
//! una caja, sea cual sea la forma del nodo. Un actor con su monigote
//! necesita sitio propio por encima de la caja, y eso toca el reparto.

use std::fmt::Write as _;

use crate::model::Rect;
use crate::model::lane_geometry::{
    self, Arrow, Bar, Frame, Lane, LaneLayout, NoteBox, SELF_LOOP_WIDTH,
};

use super::svg::{escape, n, text_block};
use super::{RenderInput, RenderTheme};

/// Radio de las esquinas de la caja de un participante.
const HEADER_RADIUS: f64 = 4.0;
/// Alto del rotulo de la esquina de un marco.
const TAG_HEIGHT: f64 = 19.0;
/// Aire alrededor del texto del rotulo de un marco.
const TAG_PADDING: f64 = 8.0;
/// Lo que mide el pico doblado de una nota.
const NOTE_FOLD: f64 = 9.0;
/// Brazo de la cruz que cierra una linea de vida destruida.
const CROSS_ARM: f64 = 8.0;
/// Separacion entre la punta de la flecha y el texto de su mensaje.
const TEXT_GAP: f64 = 4.0;

/// Reparte el diagrama con las medidas que trae la entrada.
pub fn layout_of(input: &RenderInput) -> LaneLayout {
    let participants: Vec<String> = input.document.nodes.iter().map(|n| n.id.clone()).collect();
    let empty = Default::default();
    let script = input.document.script.as_ref().unwrap_or(&empty);

    lane_geometry::layout(&participants, script, &input.measures)
}

/// Area que ocupa el diagrama. Siempre empieza en el origen: en una secuencia
/// las coordenadas las pone el reparto, no el usuario.
pub fn viewport(layout: &LaneLayout, padding: f64) -> Rect {
    Rect::new(0.0, 0.0, layout.size.width, layout.size.height).inflate(padding)
}

pub fn render(input: &RenderInput, layout: &LaneLayout) -> String {
    let theme = &input.theme;
    let mut out = String::with_capacity(4096);

    // De abajo a arriba: los marcos son el fondo de todo lo que encierran, y
    // las lineas de vida van por detras de las barras y las flechas.
    for frame in &layout.frames {
        out.push_str(&render_frame(frame, input));
    }
    for lane in &layout.lanes {
        out.push_str(&render_lifeline(lane, theme));
    }
    for bar in &layout.bars {
        out.push_str(&render_bar(bar, theme));
    }
    for lane in &layout.lanes {
        out.push_str(&render_header(lane, input));
    }
    for arrow in &layout.arrows {
        out.push_str(&render_arrow(arrow, input));
    }
    for note in &layout.notes {
        out.push_str(&render_note(note, input));
    }

    out
}

/* --- Participantes ------------------------------------------------------- */

fn render_lifeline(lane: &Lane, theme: &RenderTheme) -> String {
    let mut out = format!(
        r#"<line x1="{x}" y1="{top}" x2="{x}" y2="{bottom}" stroke="{stroke}" stroke-width="1" stroke-dasharray="4 4"/>"#,
        x = n(lane.x),
        top = n(lane.top),
        bottom = n(lane.bottom),
        stroke = escape(&theme.node_stroke),
    );

    // Una linea de vida destruida acaba en una cruz, no en el aire.
    if lane.destroyed {
        let _ = write!(
            out,
            r#"<path d="M {a} {b} L {c} {d} M {c} {b} L {a} {d}" stroke="{stroke}" stroke-width="1.5" fill="none"/>"#,
            a = n(lane.x - CROSS_ARM),
            b = n(lane.bottom - CROSS_ARM),
            c = n(lane.x + CROSS_ARM),
            d = n(lane.bottom + CROSS_ARM),
            stroke = escape(&theme.node_stroke),
        );
    }

    out
}

fn render_header(lane: &Lane, input: &RenderInput) -> String {
    let theme = &input.theme;
    let node = input.document.node(&lane.node);
    let style = node.map(|n| &n.style);

    let fill = style
        .and_then(|s| s.fill.as_deref())
        .unwrap_or(&theme.node_fill);
    let stroke = style
        .and_then(|s| s.stroke.as_deref())
        .unwrap_or(&theme.node_stroke);

    let mut out = format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#,
        x = n(lane.header.x),
        y = n(lane.header.y),
        w = n(lane.header.width),
        h = n(lane.header.height),
        r = n(HEADER_RADIUS),
        fill = escape(fill),
        stroke = escape(stroke),
    );

    if let Some(label) = input.labels.get(&lane.node)
        && !label.is_empty()
    {
        let color = label
            .color
            .as_deref()
            .or(style.and_then(|s| s.text_color.as_deref()))
            .unwrap_or(&theme.node_text);

        out.push_str(&text_block(
            label,
            lane.header.center().x,
            lane.header.center().y - label.height() / 2.0,
            color,
            theme,
        ));
    }

    out
}

fn render_bar(bar: &Bar, theme: &RenderTheme) -> String {
    format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
        x = n(bar.rect.x),
        y = n(bar.rect.y),
        w = n(bar.rect.width),
        h = n(bar.rect.height),
        fill = escape(&theme.node_fill),
        stroke = escape(&theme.node_stroke),
    )
}

/* --- Mensajes ------------------------------------------------------------ */

fn render_arrow(arrow: &Arrow, input: &RenderInput) -> String {
    let theme = &input.theme;
    let dashed = if arrow.kind.is_dashed() {
        r#" stroke-dasharray="6 4""#
    } else {
        ""
    };
    let marker = if arrow.kind.is_filled() {
        "dm-arrow-filled"
    } else {
        "dm-arrow-open"
    };

    let mut out = String::new();

    match arrow.loop_height {
        // Un mensaje a uno mismo sale, baja y vuelve.
        Some(height) => {
            let right = arrow.from_x + SELF_LOOP_WIDTH;
            let _ = write!(
                out,
                r#"<path d="M {x} {y} L {right} {y} L {right} {down} L {x} {down}" fill="none" stroke="{stroke}" stroke-width="1.5"{dashed} marker-end="url(#{marker})"/>"#,
                x = n(arrow.from_x),
                y = n(arrow.y),
                right = n(right),
                down = n(arrow.y + height),
                stroke = escape(&theme.edge_stroke),
            );
        }
        None => {
            let _ = write!(
                out,
                r#"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{stroke}" stroke-width="1.5"{dashed} marker-end="url(#{marker})"/>"#,
                x1 = n(arrow.from_x),
                x2 = n(arrow.to_x),
                y = n(arrow.y),
                stroke = escape(&theme.edge_stroke),
            );
        }
    }

    // Un mensaje de creacion apunta a la caja del creado, que ya se dibuja
    // sola; lo demas se rotula igual.
    if let Some(label) = input.labels.get(&arrow.id)
        && !label.is_empty()
    {
        let color = label.color.as_deref().unwrap_or(&theme.edge_text);
        out.push_str(&text_block(
            label,
            arrow.text_at.x + label.width() / 2.0,
            arrow.text_at.y,
            color,
            theme,
        ));
    }

    out
}

/* --- Marcos -------------------------------------------------------------- */

fn render_frame(frame: &Frame, input: &RenderInput) -> String {
    let theme = &input.theme;
    let rect = frame.rect;

    let mut out = format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="{stroke}" stroke-width="1"/>"#,
        x = n(rect.x),
        y = n(rect.y),
        w = n(rect.width),
        h = n(rect.height),
        stroke = escape(&theme.node_stroke),
    );

    // Rotulo de la esquina: una pestana con el corte en diagonal, como en
    // UML. Su ancho sale del texto medido.
    if let Some(label) = input.labels.get(&frame.id)
        && !label.is_empty()
    {
        let width = label.width() + TAG_PADDING * 2.0;
        let cut = 8.0;
        let _ = write!(
            out,
            r#"<path d="M {x} {y} L {right} {y} L {right} {low} L {cut_x} {bottom} L {x} {bottom} Z" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
            x = n(rect.x),
            y = n(rect.y),
            right = n(rect.x + width),
            low = n(rect.y + TAG_HEIGHT - cut),
            cut_x = n(rect.x + width - cut),
            bottom = n(rect.y + TAG_HEIGHT),
            fill = escape(&theme.background),
            stroke = escape(&theme.node_stroke),
        );

        out.push_str(&text_block(
            label,
            rect.x + width / 2.0,
            rect.y + (TAG_HEIGHT - label.height()) / 2.0,
            &theme.edge_text,
            theme,
        ));
    }

    for divider in &frame.dividers {
        let _ = write!(
            out,
            r#"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{stroke}" stroke-width="1" stroke-dasharray="4 4"/>"#,
            x1 = n(rect.x),
            x2 = n(rect.right()),
            y = n(divider.y),
            stroke = escape(&theme.node_stroke),
        );

        if let Some(label) = input.labels.get(&divider.id)
            && !label.is_empty()
        {
            out.push_str(&text_block(
                label,
                rect.x + TAG_PADDING + label.width() / 2.0,
                divider.y + TEXT_GAP,
                &theme.edge_text,
                theme,
            ));
        }
    }

    out
}

/* --- Notas --------------------------------------------------------------- */

fn render_note(note: &NoteBox, input: &RenderInput) -> String {
    let theme = &input.theme;
    let r = note.rect;

    // Caja con el pico de arriba a la derecha doblado.
    let mut out = format!(
        r#"<path d="M {x} {y} L {fold_x} {y} L {right} {fold_y} L {right} {bottom} L {x} {bottom} Z" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
        x = n(r.x),
        y = n(r.y),
        fold_x = n(r.right() - NOTE_FOLD),
        right = n(r.right()),
        fold_y = n(r.y + NOTE_FOLD),
        bottom = n(r.bottom()),
        fill = escape(&theme.node_fill),
        stroke = escape(&theme.node_stroke),
    );

    let _ = write!(
        out,
        r#"<path d="M {fold_x} {y} L {fold_x} {fold_y} L {right} {fold_y}" fill="none" stroke="{stroke}" stroke-width="1"/>"#,
        fold_x = n(r.right() - NOTE_FOLD),
        y = n(r.y),
        fold_y = n(r.y + NOTE_FOLD),
        right = n(r.right()),
        stroke = escape(&theme.node_stroke),
    );

    if let Some(label) = input.labels.get(&note.id)
        && !label.is_empty()
    {
        out.push_str(&text_block(
            label,
            r.center().x,
            r.center().y - label.height() / 2.0,
            &theme.node_text,
            theme,
        ));
    }

    out
}

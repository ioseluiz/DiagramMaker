//! Colores en la sintaxis que entiende el exportador.
//!
//! El lienzo pinta con el motor CSS del navegador y la exportacion con
//! `resvg`, que lee colores de SVG 1.1. No son el mismo analizador, y ahi
//! esta la trampa: `rgb(37 99 235 / 0.16)` —la sintaxis de CSS Color 4, con
//! espacios y barra, que es la que escribe la paleta— el navegador la
//! entiende desde hace anos y `resvg` no. Un color que no reconoce no da
//! error: lo pinta **negro**. Por eso el diagrama se veia bien en pantalla y
//! salia en negro en el PNG y en el PDF.
//!
//! Aqui se traduce a `rgba(37, 99, 235, 0.16)`, la forma clasica, que
//! entienden los dos. La traduccion se hace una sola vez al entrar en el
//! generador, sobre todos los colores a la vez, en lugar de en cada sitio
//! que escribe uno: asi un sitio nuevo no puede olvidarse de llamarla.

use std::borrow::Cow;

use super::RenderInput;

/// Copia del encargo con todos los colores traducidos.
pub fn sanitized(input: &RenderInput) -> RenderInput {
    let mut out = input.clone();

    for node in &mut out.document.nodes {
        traducir(&mut node.style.fill);
        traducir(&mut node.style.stroke);
        traducir(&mut node.style.text_color);
    }
    for edge in &mut out.document.edges {
        traducir(&mut edge.style.stroke);
        traducir(&mut edge.style.text_color);
    }
    for label in out.labels.values_mut() {
        traducir(&mut label.color);
    }

    let theme = &mut out.theme;
    for campo in [
        &mut theme.background,
        &mut theme.node_fill,
        &mut theme.node_stroke,
        &mut theme.node_text,
        &mut theme.edge_stroke,
        &mut theme.edge_text,
    ] {
        if let Cow::Owned(nuevo) = svg_safe(campo) {
            *campo = nuevo;
        }
    }

    out
}

fn traducir(campo: &mut Option<String>) {
    if let Some(valor) = campo
        && let Cow::Owned(nuevo) = svg_safe(valor)
    {
        *valor = nuevo;
    }
}

/// Pasa un color a la forma que entiende el exportador.
///
/// Lo que ya la tiene vuelve prestado, sin copiarse. Lo que no se reconoce
/// tambien: ante la duda es mejor dejarlo intacto —y que falle a la vista—
/// que inventarle un color.
pub fn svg_safe(value: &str) -> Cow<'_, str> {
    match convertido(value.trim()) {
        Some(nuevo) => Cow::Owned(nuevo),
        None => Cow::Borrowed(value),
    }
}

fn convertido(text: &str) -> Option<String> {
    let abre = text.find('(')?;

    // `rgb` y `rgba` nombran la misma funcion en CSS moderno; lo que decide
    // si el resultado lleva alfa es la barra, no el nombre.
    let (sin_alfa, con_alfa) = match text[..abre].trim().to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => ("rgb", "rgba"),
        "hsl" | "hsla" => ("hsl", "hsla"),
        _ => return None,
    };

    let dentro = text[abre + 1..].strip_suffix(')')?;

    // Con comas ya es la forma clasica: no hay nada que traducir.
    if dentro.contains(',') {
        return None;
    }

    let (componentes, alfa) = match dentro.split_once('/') {
        Some((c, a)) => (c, Some(a)),
        None => (dentro, None),
    };

    let partes: Vec<&str> = componentes.split_whitespace().collect();
    if partes.len() != 3 {
        return None;
    }
    let juntas = partes.join(", ");

    Some(match alfa {
        Some(a) => format!("{con_alfa}({juntas}, {})", fraccion(a)?),
        None => format!("{sin_alfa}({juntas})"),
    })
}

/// El alfa, siempre como fraccion. `16%` es `0.16`.
///
/// Se reescribe aunque ya venga en fraccion para normalizar formas que el
/// analizador clasico podria no aceptar, como `.16` sin el cero delante.
fn fraccion(alfa: &str) -> Option<String> {
    let (texto, escala) = match alfa.trim().strip_suffix('%') {
        Some(pct) => (pct.trim(), 100.0),
        None => (alfa.trim(), 1.0),
    };

    let valor: f64 = texto.parse().ok()?;
    if !valor.is_finite() {
        return None;
    }

    Some(format!("{}", valor / escala))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_sintaxis_moderna_pasa_a_la_clasica() {
        assert_eq!(svg_safe("rgb(37 99 235 / 0.16)"), "rgba(37, 99, 235, 0.16)");
        assert_eq!(svg_safe("rgb(37 99 235)"), "rgb(37, 99, 235)");
        assert_eq!(
            svg_safe("hsl(210 70% 50% / 0.5)"),
            "hsla(210, 70%, 50%, 0.5)"
        );
    }

    #[test]
    fn el_alfa_en_porcentaje_sale_en_fraccion() {
        assert_eq!(svg_safe("rgb(37 99 235 / 16%)"), "rgba(37, 99, 235, 0.16)");
        assert_eq!(svg_safe("rgb(0 0 0 / .5)"), "rgba(0, 0, 0, 0.5)");
    }

    #[test]
    fn lo_que_ya_estaba_bien_no_se_toca() {
        // Y ademas vuelve prestado: traducir de mas cuesta una copia por
        // color y por exportacion.
        for color in [
            "#2563eb",
            "#2563eb29",
            "rgb(37, 99, 235)",
            "rgba(37, 99, 235, 0.16)",
            "none",
            "currentColor",
            "steelblue",
        ] {
            assert!(
                matches!(svg_safe(color), Cow::Borrowed(_)),
                "`{color}` no necesitaba traduccion"
            );
        }
    }

    #[test]
    fn lo_que_no_se_entiende_se_deja_como_esta() {
        // Un degradado, una referencia o algo escrito a medias: cambiarlo
        // seria peor que dejarlo, porque taparia el fallo de quien lo puso.
        for color in [
            "url(#degradado)",
            "rgb(37 99)",
            "rgb(37 99 235 / )",
            "rgb(37 99 235",
            "var(--color-nodo)",
        ] {
            assert_eq!(svg_safe(color), color, "`{color}` no deberia cambiar");
        }
    }
}

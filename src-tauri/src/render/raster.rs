//! Rasterizado del SVG a PNG con `resvg`.
//!
//! Parte del SVG que genera `super::svg`, nunca del documento: si el PNG se
//! construyera por su cuenta, acabaria pareciendose al SVG solo por
//! casualidad.

use std::sync::OnceLock;

use resvg::tiny_skia;
use resvg::usvg;

use crate::error::{AppError, AppResult};

/// Escala maxima. Un diagrama grande a x8 son cientos de megapixeles; mas vale
/// recortar aqui que quedarse sin memoria a mitad de la exportacion.
pub const MAX_SCALE: f64 = 8.0;
/// Lado maximo del PNG resultante, en pixeles.
pub const MAX_PIXELS_PER_SIDE: u32 = 20_000;

/// Base de datos de fuentes del sistema.
///
/// Cargarla cuesta decenas de milisegundos, asi que se hace una sola vez. Se
/// usan las fuentes del sistema a proposito: el lienzo tambien dibuja con
/// ellas, de modo que ambos lados componen el texto con el mismo archivo de
/// fuente. Las medidas, en cualquier caso, llegan ya hechas desde el navegador
/// (ver `render::LabelLayout`).
fn fonts() -> &'static usvg::fontdb::Database {
    static FONTS: OnceLock<usvg::fontdb::Database> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// Convierte SVG a PNG. `scale` multiplica el tamano del diagrama.
pub fn to_png(svg: &str, scale: f64) -> AppResult<Vec<u8>> {
    let scale = scale.clamp(0.1, MAX_SCALE);

    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fonts().clone()),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|e| AppError::InvalidDocument(format!("el SVG generado no es valido: {e}")))?;

    let size = tree.size();
    let width = ((size.width() as f64) * scale).round().max(1.0) as u32;
    let height = ((size.height() as f64) * scale).round().max(1.0) as u32;

    if width > MAX_PIXELS_PER_SIDE || height > MAX_PIXELS_PER_SIDE {
        return Err(AppError::Unsupported(format!(
            "la imagen resultante seria de {width}x{height} px. Baja la escala o exporta a SVG."
        )));
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        AppError::Unsupported(format!(
            "no hay memoria para una imagen de {width}x{height} px."
        ))
    })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );

    pixmap
        .encode_png()
        .map_err(|e| AppError::Unsupported(format!("no se pudo codificar el PNG: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{fixtures, svg};

    /// Firma de un PNG: los ocho primeros bytes del formato.
    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    /// Lee ancho y alto de la cabecera IHDR.
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        (w, h)
    }

    #[test]
    fn produce_un_png_valido() {
        let png = to_png(&svg::render(&fixtures::input()), 1.0).unwrap();

        assert_eq!(&png[..8], &PNG_MAGIC);
    }

    #[test]
    fn la_escala_multiplica_el_tamano() {
        let doc = svg::render(&fixtures::input());

        let (w1, h1) = png_size(&to_png(&doc, 1.0).unwrap());
        let (w2, h2) = png_size(&to_png(&doc, 2.0).unwrap());

        assert_eq!((w2, h2), (w1 * 2, h1 * 2));
    }

    #[test]
    fn el_tamano_coincide_con_el_viewbox() {
        let input = fixtures::input();
        let view = svg::viewport(&input);

        let (w, h) = png_size(&to_png(&svg::render(&input), 1.0).unwrap());

        assert_eq!((w, h), (view.width as u32, view.height as u32));
    }

    #[test]
    fn rechaza_un_svg_mal_formado() {
        assert!(matches!(
            to_png("<svg esto no cierra", 1.0).unwrap_err(),
            AppError::InvalidDocument(_)
        ));
    }

    #[test]
    fn la_escala_se_recorta_al_maximo() {
        let doc = svg::render(&fixtures::input());

        let grande = png_size(&to_png(&doc, 1000.0).unwrap());
        let tope = png_size(&to_png(&doc, MAX_SCALE).unwrap());

        assert_eq!(grande, tope);
    }

    #[test]
    fn un_diagrama_enorme_falla_con_un_mensaje_util() {
        // 30000 px de ancho supera el limite por lado.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="30000" height="10" viewBox="0 0 30000 10"></svg>"#;

        let err = to_png(svg, 1.0).unwrap_err();

        assert!(err.to_string().contains("Baja la escala"), "{err}");
    }

    #[test]
    fn el_fondo_transparente_llega_hasta_el_png() {
        let mut input = fixtures::input();
        input.options.opaque_background = false;

        let png = to_png(&svg::render(&input), 1.0).unwrap();

        // El PNG debe declarar canal alfa: color type 6 (RGBA) en el IHDR.
        assert_eq!(png[25], 6, "se esperaba RGBA");
    }
}

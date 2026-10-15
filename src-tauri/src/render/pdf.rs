//! Conversion del SVG a PDF con `svg2pdf`.
//!
//! El PDF sale vectorial: el texto sigue siendo texto y se puede buscar y
//! seleccionar, que es lo que se espera de un diagrama incrustado en un
//! documento tecnico.

use svg2pdf::usvg;

use crate::error::{AppError, AppResult};

/// Puntos PostScript por pulgada. Las unidades del diagrama se tratan como
/// pixeles CSS a 96 ppp, que es como las interpreta el navegador.
const CSS_DPI: f32 = 96.0;
const PDF_DPI: f32 = 72.0;

pub fn to_pdf(svg: &str) -> AppResult<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|e| AppError::InvalidDocument(format!("el SVG generado no es valido: {e}")))?;

    let conversion = svg2pdf::ConversionOptions {
        // Mantener el texto como texto; si se convierte a curvas deja de poder
        // buscarse dentro del PDF.
        embed_text: true,
        ..svg2pdf::ConversionOptions::default()
    };

    let page = svg2pdf::PageOptions {
        dpi: CSS_DPI / PDF_DPI * 72.0,
    };

    svg2pdf::to_pdf(&tree, conversion, page)
        .map_err(|e| AppError::Unsupported(format!("no se pudo generar el PDF: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{fixtures, svg};

    #[test]
    fn produce_un_pdf_valido() {
        let pdf = to_pdf(&svg::render(&fixtures::input())).unwrap();

        assert_eq!(&pdf[..5], b"%PDF-");
        assert!(
            pdf.len() > 500,
            "un PDF de {} bytes parece vacio",
            pdf.len()
        );
    }

    #[test]
    fn el_pdf_termina_con_su_marca_de_fin() {
        let pdf = to_pdf(&svg::render(&fixtures::input())).unwrap();
        let tail = String::from_utf8_lossy(&pdf[pdf.len().saturating_sub(32)..]);

        assert!(tail.contains("%%EOF"), "cola inesperada: {tail}");
    }

    #[test]
    fn rechaza_un_svg_mal_formado() {
        assert!(matches!(
            to_pdf("<svg esto no cierra").unwrap_err(),
            AppError::InvalidDocument(_)
        ));
    }

    #[test]
    fn un_documento_vacio_tambien_produce_pdf() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"></svg>"#;

        assert_eq!(&to_pdf(svg).unwrap()[..5], b"%PDF-");
    }
}

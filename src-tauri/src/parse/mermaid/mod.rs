//! Importadores de Mermaid.
//!
//! Mermaid no es un lenguaje sino varios con la misma puerta de entrada: la
//! primera linea con contenido dice de que diagrama se trata y a partir de
//! ahi la sintaxis es distinta. Este modulo solo olfatea esa linea y pasa el
//! texto a quien sepa leerlo.
//!
//! Lo que no se reconoce no se adivina: se reporta con linea y columna.

pub(crate) mod class_diagram;
pub(crate) mod er_diagram;
pub(crate) mod flowchart;
pub(crate) mod sequence;
pub(crate) mod shapes;
pub(crate) mod state_diagram;

use super::ImportResult;

/// Dialectos de Mermaid que se entienden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Flowchart,
    Er,
    Sequence,
    Class,
    State,
}

/// Que clase de diagrama declara el codigo.
///
/// Se mira la primera linea con contenido, saltando comentarios. Sin
/// cabecera reconocible se asume `flowchart`, que es el dialecto mas comun y
/// el unico cuyos fragmentos se pegan a menudo sin la primera linea.
pub fn flavor_of(source: &str) -> Flavor {
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        let first = line.split_whitespace().next().unwrap_or("").to_lowercase();
        return match first.as_str() {
            "sequencediagram" => Flavor::Sequence,
            // `classDiagram-v2` es el mismo dialecto con otro nombre.
            "classdiagram" | "classdiagram-v2" => Flavor::Class,
            "statediagram" | "statediagram-v2" => Flavor::State,
            "erdiagram" => Flavor::Er,
            _ => Flavor::Flowchart,
        };
    }

    Flavor::Flowchart
}

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    match flavor_of(source) {
        Flavor::Flowchart => flowchart::import(source, now_ms),
        Flavor::Er => er_diagram::import(source, now_ms),
        Flavor::Sequence => sequence::import(source, now_ms),
        Flavor::Class => class_diagram::import(source, now_ms),
        Flavor::State => state_diagram::import(source, now_ms),
    }
}

/// Quita el comentario `%%` de una linea, respetando el que va entre comillas.
pub(crate) fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;

    for i in 0..bytes.len() {
        match bytes[i] {
            b'"' => quoted = !quoted,
            b'%' if !quoted && bytes.get(i + 1) == Some(&b'%') => return &line[..i],
            _ => {}
        }
    }

    line
}

pub use flowchart::arrow_for;
pub use shapes::delimiters_for;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DiagramKind;

    #[test]
    fn reconoce_un_diagrama_de_secuencia_por_su_cabecera() {
        assert_eq!(
            flavor_of("sequenceDiagram\n  A->>B: hola"),
            Flavor::Sequence
        );
        assert_eq!(flavor_of("  SEQUENCEDIAGRAM  "), Flavor::Sequence);
    }

    #[test]
    fn reconoce_un_diagrama_de_clases_por_su_cabecera() {
        assert_eq!(
            flavor_of(
                "classDiagram
  class A"
            ),
            Flavor::Class
        );
        assert_eq!(flavor_of("classDiagram-v2"), Flavor::Class);
    }

    #[test]
    fn reconoce_una_maquina_de_estados_por_su_cabecera() {
        assert_eq!(flavor_of("stateDiagram-v2"), Flavor::State);
        assert_eq!(flavor_of("stateDiagram"), Flavor::State);
    }

    #[test]
    fn reconoce_un_grafo_por_la_suya() {
        assert_eq!(flavor_of("flowchart LR\n  a --> b"), Flavor::Flowchart);
        assert_eq!(flavor_of("graph TD"), Flavor::Flowchart);
    }

    #[test]
    fn los_comentarios_de_cabecera_no_despistan() {
        let source = "%% un comentario\n\n%% otro\nsequenceDiagram\n  A->>B: hola";

        assert_eq!(flavor_of(source), Flavor::Sequence);
    }

    #[test]
    fn sin_cabecera_se_asume_un_grafo() {
        // Es el unico dialecto cuyos fragmentos se pegan sin la primera
        // linea con alguna frecuencia.
        assert_eq!(flavor_of("a --> b"), Flavor::Flowchart);
        assert_eq!(flavor_of(""), Flavor::Flowchart);
    }

    #[test]
    fn el_despachador_lleva_cada_uno_a_su_sitio() {
        assert_eq!(
            import("sequenceDiagram\n  A->>B: hola", 0).document.kind,
            DiagramKind::Sequence
        );
        assert_eq!(
            import("flowchart LR\n  a --> b", 0).document.kind,
            DiagramKind::Architecture
        );
    }
}

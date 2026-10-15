//! Exportacion del documento.
//!
//! Hay un solo generador —`svg`— y PNG y PDF se derivan de su salida. Asi las
//! tres exportaciones no pueden diferir entre si: o salen las tres bien, o las
//! tres igual de mal.
//!
//! Lo que **no** calcula este modulo es la geometria que solo el navegador
//! conoce: el ancho del texto ya compuesto y el trazado de las aristas que
//! dibuja React Flow. Ambos llegan medidos desde el frontend dentro de
//! `RenderInput`. Recalcularlos aqui con otra tipografia u otro algoritmo de
//! ruteo es justo lo que haria que el PNG no se pareciera al lienzo.

pub mod lane;
pub mod pdf;
pub mod raster;
pub mod svg;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{Document, Measures};

/// Una linea de texto ya compuesta por el navegador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct TextLine {
    pub text: String,
    /// Ancho medido con `canvas.measureText`, en unidades del diagrama.
    pub width: f64,
}

/// Donde se ancla el bloque de texto respecto a la caja del nodo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum LabelPlacement {
    #[default]
    Center,
    /// Debajo de la caja: el nombre de un actor.
    Below,
    /// En la solapa de un paquete.
    Tab,
}

/// Etiqueta medida y partida en lineas por el navegador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct LabelLayout {
    pub lines: Vec<TextLine>,
    pub font_size: f64,
    pub line_height: f64,
    #[serde(default)]
    pub placement: LabelPlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

impl LabelLayout {
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.text.trim().is_empty())
    }

    pub fn width(&self) -> f64 {
        self.lines.iter().map(|l| l.width).fold(0.0, f64::max)
    }

    pub fn height(&self) -> f64 {
        self.lines.len() as f64 * self.line_height
    }
}

/// Un cruce por el que esta arista pasa por encima.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Hop {
    pub x: f64,
    pub y: f64,
    /// Direccion de la linea en el cruce, en radianes.
    pub angle: f64,
}

/// Radio del puente. Debe coincidir con `HOP_RADIUS` de
/// `src/canvas/graph/crossings.ts`, o el SVG exportado no se pareceria a lo
/// que se ve en pantalla.
pub const HOP_RADIUS: f64 = 8.0;

/// Trazado de una arista, tal y como lo dibujo el lienzo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct EdgePath {
    /// Atributo `d` completo, en coordenadas absolutas del diagrama.
    pub d: String,
    /// Centro de la etiqueta, si la arista tiene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label_at: Option<crate::model::Point>,
    /// Cruces donde esta arista salta por encima de otra.
    #[serde(default)]
    pub hops: Vec<Hop>,
}

/// Colores resueltos del tema activo.
///
/// Llegan desde el frontend ya resueltos a valores concretos: el lienzo pinta
/// con variables CSS, y un SVG exportado que dependiera de ellas saldria en
/// blanco fuera de la aplicacion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct RenderTheme {
    pub background: String,
    pub node_fill: String,
    pub node_stroke: String,
    pub node_text: String,
    pub edge_stroke: String,
    pub edge_text: String,
    pub font_family: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ExportOptions {
    /// Margen alrededor del contenido, en unidades del diagrama.
    #[serde(default = "default_padding")]
    pub padding: f64,
    /// `false` deja el fondo transparente.
    #[serde(default)]
    pub opaque_background: bool,
    /// Multiplicador de resolucion para PNG. 1.0 = tamano del diagrama.
    #[serde(default = "default_scale")]
    pub scale: f64,
}

fn default_padding() -> f64 {
    24.0
}

fn default_scale() -> f64 {
    2.0
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            padding: default_padding(),
            opaque_background: true,
            scale: default_scale(),
        }
    }
}

/// Todo lo que hace falta para exportar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct RenderInput {
    pub document: Document,
    /// Etiquetas medidas, indexadas por id de nodo o de arista.
    #[serde(default)]
    pub labels: HashMap<String, LabelLayout>,
    /// Trazados de arista, indexados por id de arista.
    #[serde(default)]
    pub edges: HashMap<String, EdgePath>,
    pub theme: RenderTheme,
    #[serde(default)]
    pub options: ExportOptions,
    /// Medidas que necesita el reparto de un diagrama de secuencia. Vacias
    /// en los demas, que traen la geometria ya puesta en el documento.
    #[serde(default)]
    pub measures: Measures,
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::model::{DiagramKind, Edge, EdgeKind, Node, NodeShape, Point};

    pub fn theme() -> RenderTheme {
        RenderTheme {
            background: "#ffffff".into(),
            node_fill: "#ffffff".into(),
            node_stroke: "#b6bec9".into(),
            node_text: "#16191d".into(),
            edge_stroke: "#b6bec9".into(),
            edge_text: "#5b6471".into(),
            font_family: "Segoe UI, system-ui, sans-serif".into(),
        }
    }

    pub fn label(text: &str, width: f64) -> LabelLayout {
        LabelLayout {
            lines: vec![TextLine {
                text: text.into(),
                width,
            }],
            font_size: 13.0,
            line_height: 17.0,
            placement: LabelPlacement::Center,
            color: None,
            bold: false,
            italic: false,
        }
    }

    /// Dos nodos unidos por una arista con etiqueta.
    pub fn input() -> RenderInput {
        let mut document = Document::new("d1", DiagramKind::Architecture, 0);
        document
            .nodes
            .push(Node::new("a", "API", NodeShape::Rectangle).at(0.0, 0.0, 160.0, 60.0));
        document
            .nodes
            .push(Node::new("b", "Base de datos", NodeShape::Cylinder).at(300.0, 0.0, 120.0, 90.0));
        document
            .edges
            .push(Edge::new("e1", "a", "b", EdgeKind::Flow).with_label("consulta"));

        let labels = HashMap::from([
            ("a".to_string(), label("API", 26.0)),
            ("b".to_string(), label("Base de datos", 82.0)),
            ("e1".to_string(), label("consulta", 48.0)),
        ]);

        let edges = HashMap::from([(
            "e1".to_string(),
            EdgePath {
                d: "M 160 30 L 300 45".into(),
                label_at: Some(Point::new(230.0, 37.5)),
                hops: Vec::new(),
            },
        )]);

        RenderInput {
            document,
            labels,
            edges,
            theme: theme(),
            options: ExportOptions::default(),
            measures: Default::default(),
        }
    }
}

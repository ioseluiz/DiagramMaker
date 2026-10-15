use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::geometry::Point;
use super::style::EdgeStyle;

/// Semantica de la relacion. Determina los extremos (punta de flecha, rombo)
/// y el trazo por defecto; el renderizador los traduce a geometria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum EdgeKind {
    /// Linea simple sin direccion.
    Association,
    /// Flecha abierta.
    Directed,
    /// Flecha abierta punteada.
    Dependency,
    /// Triangulo hueco: herencia / generalizacion.
    Inheritance,
    /// Triangulo hueco punteado: realizacion de interfaz.
    Realization,
    /// Rombo relleno en el origen.
    Composition,
    /// Rombo hueco en el origen.
    Aggregation,
    /// Flujo de datos o de control.
    Flow,
}

impl EdgeKind {
    pub fn is_dashed_by_default(self) -> bool {
        matches!(self, EdgeKind::Dependency | EdgeKind::Realization)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Routing {
    /// Segmentos en angulo recto. Es lo correcto para arquitectura y ER.
    #[default]
    Orthogonal,
    Straight,
    Bezier,
}

/// Un extremo de la arista.
///
/// `label` es el texto pegado a ese extremo concreto: la cardinalidad en ER
/// (`1`, `0..*`) o la multiplicidad en diagramas de clases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Endpoint {
    pub node: String,

    /// Punto de anclaje concreto dentro del nodo: en ER identifica la columna.
    /// `None` = el borde del nodo, que el ruteo elige.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Endpoint {
    pub fn new(node: impl Into<String>) -> Self {
        Endpoint {
            node: node.into(),
            port: None,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_port(mut self, port: impl Into<String>) -> Self {
        self.port = Some(port.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Edge {
    pub id: String,
    pub source: Endpoint,
    pub target: Endpoint,
    pub kind: EdgeKind,

    /// Etiqueta central de la relacion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(default)]
    pub routing: Routing,

    /// Puntos intermedios fijados por el usuario. `None` = ruteo automatico.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waypoints: Option<Vec<Point>>,

    #[serde(default, skip_serializing_if = "EdgeStyle::is_default")]
    pub style: EdgeStyle,
}

impl Edge {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        kind: EdgeKind,
    ) -> Self {
        Edge {
            id: id.into(),
            source: Endpoint::new(source),
            target: Endpoint::new(target),
            kind,
            label: None,
            routing: Routing::default(),
            waypoints: None,
            style: EdgeStyle::default(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// `true` si la arista empieza y termina en el mismo nodo. El ruteo las
    /// trata aparte porque un bucle no tiene camino entre dos cajas.
    pub fn is_self_loop(&self) -> bool {
        self.source.node == self.target.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_los_bucles_sobre_si_mismo() {
        assert!(Edge::new("e1", "a", "a", EdgeKind::Association).is_self_loop());
        assert!(!Edge::new("e2", "a", "b", EdgeKind::Association).is_self_loop());
    }

    #[test]
    fn dependencia_y_realizacion_van_punteadas() {
        assert!(EdgeKind::Dependency.is_dashed_by_default());
        assert!(EdgeKind::Realization.is_dashed_by_default());
        assert!(!EdgeKind::Inheritance.is_dashed_by_default());
    }

    #[test]
    fn el_ruteo_por_defecto_es_ortogonal() {
        assert_eq!(
            Edge::new("e1", "a", "b", EdgeKind::Flow).routing,
            Routing::Orthogonal
        );
    }

    #[test]
    fn las_cardinalidades_sobreviven_al_viaje_por_json() {
        let mut e = Edge::new("e1", "users", "orders", EdgeKind::Association);
        e.source = Endpoint::new("users").with_label("1");
        e.target = Endpoint::new("orders").with_label("0..*");

        let back: Edge = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();

        assert_eq!(back, e);
        assert_eq!(back.target.label.as_deref(), Some("0..*"));
    }
}

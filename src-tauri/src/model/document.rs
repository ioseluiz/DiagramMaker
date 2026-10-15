use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::edge::Edge;
use super::geometry::Rect;
use super::node::Node;

/// Version del esquema del documento `.dgm`. Se incrementa en cada cambio
/// incompatible del IR; `io::load` la usa para decidir si hay que migrar.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum DiagramKind {
    Er,
    Architecture,
    Class,
    UseCase,
    Sequence,
    Activity,
    State,
}

impl DiagramKind {
    /// Los diagramas de secuencia derivan su geometria al 100% del guion de
    /// mensajes, asi que no pasan ni por React Flow ni por el auto-layout de
    /// grafo: los dibuja el renderizador de carriles.
    pub fn is_lane_based(self) -> bool {
        matches!(self, DiagramKind::Sequence)
    }

    pub fn title(self) -> &'static str {
        match self {
            DiagramKind::Er => "Entidad-relacion",
            DiagramKind::Architecture => "Arquitectura",
            DiagramKind::Class => "Clases",
            DiagramKind::UseCase => "Casos de uso",
            DiagramKind::Sequence => "Secuencia",
            DiagramKind::Activity => "Actividad",
            DiagramKind::State => "Estado",
        }
    }
}

/// Como se decide la posicion de los nodos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum LayoutMode {
    /// El motor coloca todo salvo los nodos con `pinned = true`. Es el modo de
    /// un documento recien importado desde codigo.
    #[default]
    Auto,
    /// Las posiciones guardadas mandan; el motor no vuelve a correr solo.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Meta {
    pub title: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Milisegundos desde el epoch Unix. Se guarda como numero y no como texto
    /// ISO para no arrastrar una dependencia de fechas al backend; el frontend
    /// lo formatea con `Intl`.
    ///
    /// `ts-rs` mapearia `i64` a `bigint`, que no es lo que llega por JSON ni
    /// lo que devuelve `Date.now()`. Una marca de tiempo en milisegundos cabe
    /// de sobra en el entero seguro de JavaScript (2^53), asi que se declara
    /// como `number`.
    #[ts(type = "number")]
    pub created: i64,
    #[ts(type = "number")]
    pub modified: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Document {
    pub schema_version: u32,
    pub id: String,
    pub kind: DiagramKind,
    pub meta: Meta,

    #[serde(default)]
    pub nodes: Vec<Node>,

    #[serde(default)]
    pub edges: Vec<Edge>,

    #[serde(default)]
    pub layout: LayoutMode,
}

impl Document {
    pub fn new(id: impl Into<String>, kind: DiagramKind, now_ms: i64) -> Self {
        Document {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            kind,
            meta: Meta {
                title: kind.title().to_string(),
                description: None,
                created: now_ms,
                modified: now_ms,
            },
            nodes: Vec::new(),
            edges: Vec::new(),
            layout: LayoutMode::default(),
        }
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn edge(&self, id: &str) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == id)
    }

    /// Hijos directos de un contenedor. `None` devuelve los nodos raiz.
    pub fn children_of<'a>(&'a self, parent: Option<&str>) -> impl Iterator<Item = &'a Node> {
        self.nodes
            .iter()
            .filter(move |n| n.parent.as_deref() == parent)
    }

    /// `true` si aun falta colocar algun nodo. El frontend lo consulta para
    /// decidir si debe correr el auto-layout al abrir o al importar.
    pub fn needs_layout(&self) -> bool {
        self.nodes.iter().any(|n| n.geometry.is_none())
    }

    /// Caja que envuelve a todos los nodos colocados. `None` si el documento
    /// esta vacio o no tiene geometria todavia. Es la base del `viewBox` al
    /// exportar.
    pub fn bounds(&self) -> Option<Rect> {
        self.nodes
            .iter()
            .filter_map(|n| n.geometry)
            .reduce(|acc, r| acc.union(&r))
    }

    pub fn touch(&mut self, now_ms: i64) {
        self.meta.modified = now_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::node::NodeShape;

    fn doc() -> Document {
        Document::new("d1", DiagramKind::Architecture, 1_700_000_000_000)
    }

    #[test]
    fn un_documento_nuevo_toma_el_titulo_de_su_tipo() {
        assert_eq!(doc().meta.title, "Arquitectura");
    }

    #[test]
    fn un_documento_vacio_no_necesita_layout_ni_tiene_limites() {
        let d = doc();

        assert!(!d.needs_layout());
        assert_eq!(d.bounds(), None);
    }

    #[test]
    fn un_nodo_sin_geometria_hace_que_el_documento_necesite_layout() {
        let mut d = doc();
        d.nodes.push(Node::new("a", "A", NodeShape::Rectangle));

        assert!(d.needs_layout());
        assert_eq!(d.bounds(), None);
    }

    #[test]
    fn los_limites_envuelven_a_todos_los_nodos_colocados() {
        let mut d = doc();
        d.nodes
            .push(Node::new("a", "A", NodeShape::Rectangle).at(0.0, 0.0, 160.0, 60.0));
        d.nodes
            .push(Node::new("b", "B", NodeShape::Rectangle).at(300.0, 200.0, 160.0, 60.0));

        assert_eq!(d.bounds(), Some(Rect::new(0.0, 0.0, 460.0, 260.0)));
    }

    #[test]
    fn children_of_separa_raiz_de_contenido() {
        let mut d = doc();
        d.nodes
            .push(Node::new("pkg", "Paquete", NodeShape::Package));
        d.nodes
            .push(Node::new("a", "A", NodeShape::Rectangle).with_parent("pkg"));

        let raiz: Vec<_> = d.children_of(None).map(|n| n.id.as_str()).collect();
        let dentro: Vec<_> = d.children_of(Some("pkg")).map(|n| n.id.as_str()).collect();

        assert_eq!(raiz, ["pkg"]);
        assert_eq!(dentro, ["a"]);
    }

    #[test]
    fn solo_secuencia_usa_el_renderizador_de_carriles() {
        assert!(DiagramKind::Sequence.is_lane_based());
        assert!(!DiagramKind::Er.is_lane_based());
        assert!(!DiagramKind::Architecture.is_lane_based());
    }
}

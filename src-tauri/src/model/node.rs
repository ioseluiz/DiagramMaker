use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::er::ErTable;
use super::geometry::Rect;
use super::style::NodeStyle;

/// Forma visual de la caja. Es puramente grafica: la semantica (que representa
/// el nodo dentro de su tipo de diagrama) vive en `NodeData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum NodeShape {
    Rectangle,
    RoundedRect,
    Ellipse,
    Diamond,
    Cylinder,
    Note,
    Package,
    Actor,
    /// Hexagono: preparacion en un diagrama de flujo, sistema externo en C4.
    Hexagon,
    /// Paralelogramo: entrada o salida de datos.
    Parallelogram,
    /// Texto suelto, sin recuadro: una anotacion sobre el lienzo.
    Text,
    /// Tabla de un diagrama entidad-relacion: cabecera mas una fila por
    /// columna. Su contorno depende de cuantas columnas tenga, no solo de la
    /// caja, asi que se dibuja aparte del resto de formas.
    Table,
}

impl NodeShape {
    /// `true` si el contorno depende de los datos del nodo y no solo de su
    /// tamano.
    pub fn is_data_driven(self) -> bool {
        matches!(self, NodeShape::Table)
    }
}

/// Datos propios del tipo de diagrama.
///
/// Punto de extension del IR: F3 anade `Er { columns }`, F6 anade
/// `Class { attributes, operations }`, etc. `Plain` cubre los nodos que solo
/// tienen etiqueta.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum NodeData {
    #[default]
    Plain,
    /// Tabla de un diagrama entidad-relacion.
    Er(ErTable),
}

impl NodeData {
    pub fn as_er(&self) -> Option<&ErTable> {
        match self {
            NodeData::Er(table) => Some(table),
            NodeData::Plain => None,
        }
    }

    pub fn as_er_mut(&mut self) -> Option<&mut ErTable> {
        match self {
            NodeData::Er(table) => Some(table),
            NodeData::Plain => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Node {
    pub id: String,
    pub label: String,

    /// Nodo contenedor, para anidamiento (paquetes, contenedores C4).
    /// `None` = esta en la raiz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    pub shape: NodeShape,

    /// `None` mientras el auto-layout no se ha ejecutado. Un documento recien
    /// importado llega entero sin geometria.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Rect>,

    /// El usuario lo coloco a mano: el auto-layout debe respetar su posicion.
    #[serde(default)]
    pub pinned: bool,

    #[serde(default, skip_serializing_if = "NodeStyle::is_default")]
    pub style: NodeStyle,

    #[serde(default)]
    pub data: NodeData,
}

impl Node {
    pub fn new(id: impl Into<String>, label: impl Into<String>, shape: NodeShape) -> Self {
        Node {
            id: id.into(),
            label: label.into(),
            parent: None,
            shape,
            geometry: None,
            pinned: false,
            style: NodeStyle::default(),
            data: NodeData::Plain,
        }
    }

    /// Coloca el nodo. Las medidas son explicitas a proposito: los tamanos por
    /// defecto son cosa del lienzo (`src/model/shapes.ts`), que es quien mide
    /// el texto, y no deben existir tambien aqui.
    pub fn at(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.geometry = Some(Rect::new(x, y, width, height));
        self
    }

    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Construye una tabla ER ya con su forma puesta.
    pub fn table(id: impl Into<String>, name: impl Into<String>, table: ErTable) -> Self {
        let mut node = Node::new(id, name, NodeShape::Table);
        node.data = NodeData::Er(table);
        node
    }

    pub fn er(&self) -> Option<&ErTable> {
        self.data.as_er()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nodo_recien_creado_no_tiene_geometria() {
        let n = Node::new("n1", "Usuarios", NodeShape::Rectangle);

        assert!(n.geometry.is_none());
        assert!(!n.pinned);
    }

    #[test]
    fn at_fija_la_geometria_completa() {
        let n = Node::new("n1", "Decision", NodeShape::Diamond).at(10.0, 20.0, 120.0, 80.0);
        let g = n.geometry.expect("at() debe fijar geometria");

        assert_eq!((g.x, g.y, g.width, g.height), (10.0, 20.0, 120.0, 80.0));
    }

    #[test]
    fn el_estilo_por_defecto_no_se_serializa() {
        let n = Node::new("n1", "A", NodeShape::Rectangle);
        let json = serde_json::to_string(&n).unwrap();

        assert!(!json.contains("style"), "json inesperado: {json}");
        assert!(!json.contains("parent"), "json inesperado: {json}");
    }

    #[test]
    fn el_estilo_modificado_si_se_serializa_y_vuelve_igual() {
        let mut n = Node::new("n1", "A", NodeShape::Rectangle);
        n.style.fill = Some("#eee".into());

        let json = serde_json::to_string(&n).unwrap();
        let back: Node = serde_json::from_str(&json).unwrap();

        assert_eq!(back, n);
    }
}

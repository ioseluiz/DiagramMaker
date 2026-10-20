//! Representacion intermedia (IR) del documento.
//!
//! Es la fuente unica de verdad del esquema: los tipos TypeScript de
//! `src/model/generated/` se generan desde aqui con `ts-rs` y no se editan a
//! mano. Tanto el lienzo interactivo como los importadores de codigo producen
//! este mismo `Document`, y tanto la exportacion a SVG como los generadores de
//! codigo lo consumen.

pub mod class_box;
pub mod diagnostic;
pub mod document;
pub mod edge;
pub mod er;
pub mod geometry;
pub mod lane_geometry;
pub mod node;
pub mod sequence;
pub mod shape_geometry;
pub mod style;
pub mod validate;

pub use class_box::{ClassBox, ClassMember, Visibility};
pub use diagnostic::{Diagnostic, Report, Severity, Span};
pub use document::{DiagramKind, Document, LayoutDirection, LayoutMode, Meta, SCHEMA_VERSION};
pub use edge::{Edge, EdgeKind, Endpoint, Routing, Side};
pub use er::{Cardinality, ErColumn, ErTable};
pub use geometry::{Point, Rect, Size};
pub use lane_geometry::{LaneLayout, Measures};
pub use node::{Node, NodeData, NodeShape};
pub use sequence::{
    Block, BlockKind, Branch, Message, MessageKind, NotePlacement, SequenceNote, SequenceScript,
    Step,
};
pub use shape_geometry::{Primitive, primitives};
pub use style::{EdgeStyle, NodeStyle};
pub use validate::validate;

//! Representacion intermedia (IR) del documento.
//!
//! Es la fuente unica de verdad del esquema: los tipos TypeScript de
//! `src/model/generated/` se generan desde aqui con `ts-rs` y no se editan a
//! mano. Tanto el lienzo interactivo como los importadores de codigo producen
//! este mismo `Document`, y tanto la exportacion a SVG como los generadores de
//! codigo lo consumen.

pub mod diagnostic;
pub mod document;
pub mod edge;
pub mod geometry;
pub mod node;
pub mod shape_geometry;
pub mod style;
pub mod validate;

pub use diagnostic::{Diagnostic, Report, Severity, Span};
pub use document::{DiagramKind, Document, LayoutMode, Meta, SCHEMA_VERSION};
pub use edge::{Edge, EdgeKind, Endpoint, Routing};
pub use geometry::{Point, Rect, Size};
pub use node::{Node, NodeData, NodeShape};
pub use shape_geometry::{Primitive, primitives};
pub use style::{EdgeStyle, NodeStyle};
pub use validate::validate;

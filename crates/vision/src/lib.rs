pub mod model;
pub mod pipeline;

pub use model::{BoundingBox, Diagram, Edge, Node, ShapeKind};
pub use pipeline::{ShapeCandidate, build_diagram, detect_shapes};

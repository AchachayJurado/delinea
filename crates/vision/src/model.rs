/// The kinds of hand-drawn shapes v1 recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Rectangle,
    Circle,
    Diamond,
}

/// Axis-aligned bounding box in source-image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: usize,
    pub shape: ShapeKind,
    pub label: Option<String>,
    pub bbox: BoundingBox,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub directed: bool,
}

/// The recognized diagram structure, independent of D2 or any rendering concern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagram {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

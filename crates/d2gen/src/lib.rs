use vision::{Diagram, ShapeKind};

fn d2_shape_keyword(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Rectangle => "rectangle",
        ShapeKind::Circle => "circle",
        ShapeKind::Diamond => "diamond",
    }
}

fn node_id(id: usize) -> String {
    format!("node{id}")
}

/// Renders a recognized `Diagram` as D2 source text.
pub fn generate(diagram: &Diagram) -> String {
    let mut out = String::new();

    for node in &diagram.nodes {
        let id = node_id(node.id);
        let label = node.label.as_deref().unwrap_or(&id);
        out.push_str(&format!(
            "{}: {} {{shape: {}}}\n",
            id,
            label,
            d2_shape_keyword(node.shape)
        ));
    }

    for edge in &diagram.edges {
        let arrow = if edge.directed { "->" } else { "--" };
        out.push_str(&format!(
            "{} {} {}",
            node_id(edge.from),
            arrow,
            node_id(edge.to)
        ));
        if let Some(label) = &edge.label {
            out.push_str(&format!(": {label}"));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vision::{BoundingBox, Edge, Node};

    fn bbox() -> BoundingBox {
        BoundingBox {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        }
    }

    #[test]
    fn renders_shapes_and_a_directed_labeled_edge() {
        let diagram = Diagram {
            nodes: vec![
                Node {
                    id: 0,
                    shape: ShapeKind::Rectangle,
                    label: Some("Login".to_string()),
                    bbox: bbox(),
                },
                Node {
                    id: 1,
                    shape: ShapeKind::Circle,
                    label: Some("Database".to_string()),
                    bbox: bbox(),
                },
            ],
            edges: vec![Edge {
                from: 0,
                to: 1,
                label: Some("writes".to_string()),
                directed: true,
            }],
        };

        let d2 = generate(&diagram);
        assert_eq!(
            d2,
            "node0: Login {shape: rectangle}\n\
             node1: Database {shape: circle}\n\
             node0 -> node1: writes\n"
        );
    }
}

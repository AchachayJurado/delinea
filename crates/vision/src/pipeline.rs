use image::GrayImage;
use imageproc::contours::find_contours;
use imageproc::contrast::{ThresholdType, threshold};
use imageproc::point::Point;

use crate::model::{BoundingBox, Diagram, Node, ShapeKind};

/// Drops contours smaller than this (pixel area) as noise — dust, marker-cap
/// specks, JPEG artifacts.
const MIN_SHAPE_AREA_PX: f64 = 20.0;

/// Drops contours covering more than this fraction of the frame — a traced
/// background/border, not a drawn shape.
const MAX_SHAPE_AREA_FRACTION: f64 = 0.8;

/// A contour is either a recognized shape or noise to discard. Classification
/// is by solidity (contour area / bounding-box area), which is analytically
/// well separated for the three recognized shapes when filled: rectangle
/// (axis-aligned) ~1.0, circle ~pi/4 ~= 0.785, diamond (bbox-inscribed
/// rhombus) ~0.5. Anything outside these bands, or too small/too large, is
/// noise — this is what keeps whiteboard glare, texture, and stray marks
/// from turning into spurious diagram nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    Shape(ShapeKind),
    Noise,
}

fn classify_contour(points: &[Point<i32>], bbox: BoundingBox, image_area: f64) -> Classification {
    let bbox_area = f64::from(bbox.width) * f64::from(bbox.height);
    if bbox_area <= 0.0 {
        return Classification::Noise;
    }

    let area = polygon_area(points);
    if area < MIN_SHAPE_AREA_PX || (image_area > 0.0 && area / image_area > MAX_SHAPE_AREA_FRACTION)
    {
        return Classification::Noise;
    }

    let solidity = area / bbox_area;
    if solidity >= 0.85 {
        Classification::Shape(ShapeKind::Rectangle)
    } else if solidity >= 0.65 {
        Classification::Shape(ShapeKind::Circle)
    } else if solidity >= 0.35 {
        Classification::Shape(ShapeKind::Diamond)
    } else {
        Classification::Noise
    }
}

/// Polygon area via the shoelace formula.
fn polygon_area(points: &[Point<i32>]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let n = points.len();
    let mut sum = 0i64;
    for i in 0..n {
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        sum += i64::from(p1.x) * i64::from(p2.y) - i64::from(p2.x) * i64::from(p1.y);
    }
    (sum as f64).abs() / 2.0
}

fn bounding_box(points: &[Point<i32>]) -> Option<BoundingBox> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    if min_x > max_x || min_y > max_y {
        return None;
    }
    Some(BoundingBox {
        x: min_x as u32,
        y: min_y as u32,
        width: (max_x - min_x) as u32,
        height: (max_y - min_y) as u32,
    })
}

/// A recognized shape's bounding box and kind, prior to being assigned a
/// diagram node id or label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeCandidate {
    pub bbox: BoundingBox,
    pub shape: ShapeKind,
}

/// Binarizes the frame, extracts contours, and classifies each into a
/// recognized shape or discards it as noise.
pub fn detect_shapes(gray: &GrayImage) -> Vec<ShapeCandidate> {
    let binary = threshold(gray, 128, ThresholdType::Binary);
    let image_area = f64::from(gray.width()) * f64::from(gray.height());

    find_contours::<i32>(&binary)
        .iter()
        .filter_map(|contour| {
            let bbox = bounding_box(&contour.points)?;
            match classify_contour(&contour.points, bbox, image_area) {
                Classification::Shape(shape) => Some(ShapeCandidate { bbox, shape }),
                Classification::Noise => None,
            }
        })
        .collect()
}

/// Builds a `Diagram` from detected shapes. Edge/connector detection and OCR
/// labels are not implemented yet (see M2/M3 in CLAUDE.md), so every node has
/// no label and there are no edges.
pub fn build_diagram(gray: &GrayImage) -> Diagram {
    let nodes = detect_shapes(gray)
        .into_iter()
        .enumerate()
        .map(|(id, candidate)| Node {
            id,
            shape: candidate.shape,
            label: None,
            bbox: candidate.bbox,
        })
        .collect();
    Diagram {
        nodes,
        edges: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    fn blank(width: u32, height: u32) -> GrayImage {
        GrayImage::from_pixel(width, height, Luma([0]))
    }

    fn fill_rect(img: &mut GrayImage, x: u32, y: u32, w: u32, h: u32) {
        for py in y..y + h {
            for px in x..x + w {
                img.put_pixel(px, py, Luma([255]));
            }
        }
    }

    fn fill_circle(img: &mut GrayImage, cx: i32, cy: i32, r: i32) {
        for py in (cy - r).max(0)..(cy + r).min(img.height() as i32) {
            for px in (cx - r).max(0)..(cx + r).min(img.width() as i32) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r * r {
                    img.put_pixel(px as u32, py as u32, Luma([255]));
                }
            }
        }
    }

    fn fill_diamond(img: &mut GrayImage, cx: i32, cy: i32, r: i32) {
        for py in (cy - r).max(0)..(cy + r).min(img.height() as i32) {
            for px in (cx - r).max(0)..(cx + r).min(img.width() as i32) {
                let dx = (px - cx).abs();
                let dy = (py - cy).abs();
                if dx + dy <= r {
                    img.put_pixel(px as u32, py as u32, Luma([255]));
                }
            }
        }
    }

    /// Scatters small noise speckles (1-3px) across the image, simulating
    /// whiteboard glare/dust/texture that isn't part of a drawn shape.
    fn scatter_speckles(img: &mut GrayImage, count: u32) {
        let (w, h) = (img.width(), img.height());
        for i in 0..count {
            // Deterministic pseudo-scatter so the test is reproducible.
            let x = (i * 37 + 5) % w.max(1);
            let y = (i * 53 + 7) % h.max(1);
            img.put_pixel(x, y, Luma([255]));
            if x + 1 < w {
                img.put_pixel(x + 1, y, Luma([255]));
            }
        }
    }

    #[test]
    fn classifies_a_filled_rectangle() {
        let mut img = blank(100, 100);
        fill_rect(&mut img, 20, 20, 40, 30);

        let shapes = detect_shapes(&img);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].shape, ShapeKind::Rectangle);
    }

    #[test]
    fn classifies_a_filled_circle() {
        let mut img = blank(100, 100);
        fill_circle(&mut img, 50, 50, 25);

        let shapes = detect_shapes(&img);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].shape, ShapeKind::Circle);
    }

    #[test]
    fn classifies_a_filled_diamond() {
        let mut img = blank(100, 100);
        fill_diamond(&mut img, 50, 50, 30);

        let shapes = detect_shapes(&img);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].shape, ShapeKind::Diamond);
    }

    #[test]
    fn ignores_small_noise_speckles() {
        let mut img = blank(120, 120);
        fill_rect(&mut img, 30, 30, 40, 30);
        scatter_speckles(&mut img, 15);

        let shapes = detect_shapes(&img);
        assert_eq!(
            shapes.len(),
            1,
            "expected only the real rectangle, speckles should be filtered as noise"
        );
        assert_eq!(shapes[0].shape, ShapeKind::Rectangle);
    }

    #[test]
    fn ignores_a_near_full_frame_background_contour() {
        let mut img = blank(100, 100);
        // A border frame spanning almost the entire image (e.g. a whiteboard
        // edge or vignette), plus one real shape.
        for x in 0..100u32 {
            img.put_pixel(x, 0, Luma([255]));
            img.put_pixel(x, 99, Luma([255]));
        }
        for y in 0..100u32 {
            img.put_pixel(0, y, Luma([255]));
            img.put_pixel(99, y, Luma([255]));
        }
        fill_rect(&mut img, 40, 40, 20, 20);

        let shapes = detect_shapes(&img);
        assert!(
            shapes
                .iter()
                .all(|s| s.bbox.width < 90 && s.bbox.height < 90),
            "the near-full-frame border should not be classified as a shape"
        );
        assert!(
            shapes.iter().any(|s| s.shape == ShapeKind::Rectangle),
            "the real small rectangle should still be detected"
        );
    }

    #[test]
    fn detects_correct_shapes_in_a_noisy_realistic_scene() {
        let mut img = blank(200, 200);
        fill_rect(&mut img, 10, 10, 40, 30);
        fill_circle(&mut img, 120, 30, 20);
        fill_diamond(&mut img, 60, 130, 25);
        scatter_speckles(&mut img, 40);

        let shapes = detect_shapes(&img);
        let mut kinds: Vec<ShapeKind> = shapes.iter().map(|s| s.shape).collect();
        kinds.sort_by_key(|k| format!("{k:?}"));

        assert_eq!(
            kinds,
            vec![ShapeKind::Circle, ShapeKind::Diamond, ShapeKind::Rectangle],
            "expected exactly the three drawn shapes, noise excluded: {shapes:?}"
        );
    }

    #[test]
    fn build_diagram_has_no_labels_or_edges_yet() {
        let mut img = blank(100, 100);
        fill_rect(&mut img, 20, 20, 40, 30);

        let diagram = build_diagram(&img);
        assert_eq!(diagram.nodes.len(), 1);
        assert_eq!(diagram.nodes[0].label, None);
        assert!(diagram.edges.is_empty());
    }
}

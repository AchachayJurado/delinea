use image::GrayImage;
use imageproc::contours::find_contours;
use imageproc::contrast::{ThresholdType, threshold};
use imageproc::point::Point;

use crate::model::BoundingBox;

/// Binarizes the frame and extracts contour bounding boxes.
///
/// This is the first stage of the shape-detection pipeline: find candidate
/// regions before classifying them into `ShapeKind`s. Shape classification,
/// arrow/connector detection, and OCR are not implemented yet (see M1 in
/// CLAUDE.md).
pub fn find_shape_regions(gray: &GrayImage) -> Vec<BoundingBox> {
    let binary = threshold(gray, 128, ThresholdType::Binary);
    find_contours::<i32>(&binary)
        .iter()
        .filter_map(|contour| bounding_box(&contour.points))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    /// Draws a filled white square on a black background and checks that a
    /// contour is found whose bounding box matches the square.
    #[test]
    fn finds_a_single_rectangle_contour() {
        let mut img = GrayImage::from_pixel(64, 64, Luma([0]));
        for y in 10..30 {
            for x in 10..40 {
                img.put_pixel(x, y, Luma([255]));
            }
        }

        let regions = find_shape_regions(&img);
        assert!(
            !regions.is_empty(),
            "expected at least one contour to be found"
        );

        let square = regions
            .iter()
            .find(|b| b.width >= 25 && b.height >= 15)
            .expect("expected a contour roughly matching the drawn square");
        assert!(square.x <= 11 && square.y <= 11);
    }
}

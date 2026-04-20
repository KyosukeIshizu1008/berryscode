//! Spline/path data model with cubic Bezier math helpers.
//!
//! Each [`SplinePoint`] stores a position and two tangent handles
//! (in/out). A spline is a sequence of such points that the editor
//! connects with cubic Bezier curves.

use serde::{Deserialize, Serialize};

/// A single control point on a spline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SplinePoint {
    pub position: [f32; 3],
    #[serde(default)]
    pub tangent_in: [f32; 3],
    #[serde(default)]
    pub tangent_out: [f32; 3],
}

impl Default for SplinePoint {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            tangent_in: [0.0, 0.0, 0.0],
            tangent_out: [0.0, 0.0, 0.0],
        }
    }
}

/// Evaluate a cubic Bezier curve at parameter `t` (0..=1).
///
/// `p0` and `p3` are the endpoints; `p1` and `p2` are the control points.
pub fn evaluate_cubic_bezier(
    t: f32,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
) -> [f32; 3] {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;
    [
        uuu * p0[0] + 3.0 * uu * t * p1[0] + 3.0 * u * tt * p2[0] + ttt * p3[0],
        uuu * p0[1] + 3.0 * uu * t * p1[1] + 3.0 * u * tt * p2[1] + ttt * p3[1],
        uuu * p0[2] + 3.0 * uu * t * p1[2] + 3.0 * u * tt * p2[2] + ttt * p3[2],
    ]
}

/// Sample a full spline as a polyline.
///
/// For each pair of adjacent points the curve goes from `points[i].position`
/// through control points derived from the tangent handles to
/// `points[i+1].position`. If `closed` is true an additional segment
/// connects the last point back to the first.
///
/// Returns `segments_per_curve * num_curves + 1` samples (the final endpoint
/// is always included). An empty or single-point spline returns `points`
/// positions as-is.
pub fn sample_spline(
    points: &[SplinePoint],
    closed: bool,
    segments_per_curve: usize,
) -> Vec<[f32; 3]> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![points[0].position];
    }

    let segments_per_curve = segments_per_curve.max(1);

    let curve_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };

    let mut result = Vec::with_capacity(curve_count * segments_per_curve + 1);

    for i in 0..curve_count {
        let a = &points[i];
        let b = &points[(i + 1) % points.len()];

        let p0 = a.position;
        let p1 = [
            a.position[0] + a.tangent_out[0],
            a.position[1] + a.tangent_out[1],
            a.position[2] + a.tangent_out[2],
        ];
        let p2 = [
            b.position[0] + b.tangent_in[0],
            b.position[1] + b.tangent_in[1],
            b.position[2] + b.tangent_in[2],
        ];
        let p3 = b.position;

        for s in 0..segments_per_curve {
            let t = s as f32 / segments_per_curve as f32;
            result.push(evaluate_cubic_bezier(t, p0, p1, p2, p3));
        }
    }

    // Always include the final endpoint.
    if closed {
        result.push(points[0].position);
    } else {
        result.push(points[points.len() - 1].position);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_at_t0_returns_p0() {
        let p0 = [1.0, 2.0, 3.0];
        let p1 = [4.0, 5.0, 6.0];
        let p2 = [7.0, 8.0, 9.0];
        let p3 = [10.0, 11.0, 12.0];
        let result = evaluate_cubic_bezier(0.0, p0, p1, p2, p3);
        assert_eq!(result, p0);
    }

    #[test]
    fn bezier_at_t1_returns_p3() {
        let p0 = [1.0, 2.0, 3.0];
        let p1 = [4.0, 5.0, 6.0];
        let p2 = [7.0, 8.0, 9.0];
        let p3 = [10.0, 11.0, 12.0];
        let result = evaluate_cubic_bezier(1.0, p0, p1, p2, p3);
        for i in 0..3 {
            assert!(
                (result[i] - p3[i]).abs() < 1e-6,
                "component {} mismatch: {} vs {}",
                i,
                result[i],
                p3[i]
            );
        }
    }

    #[test]
    fn sample_spline_open_returns_correct_count() {
        let points = vec![
            SplinePoint {
                position: [0.0, 0.0, 0.0],
                tangent_in: [0.0; 3],
                tangent_out: [1.0, 0.0, 0.0],
            },
            SplinePoint {
                position: [3.0, 0.0, 0.0],
                tangent_in: [-1.0, 0.0, 0.0],
                tangent_out: [1.0, 0.0, 0.0],
            },
            SplinePoint {
                position: [6.0, 0.0, 0.0],
                tangent_in: [-1.0, 0.0, 0.0],
                tangent_out: [0.0; 3],
            },
        ];
        let segments = 10;
        let samples = sample_spline(&points, false, segments);
        // 2 curves * 10 segments + 1 endpoint = 21
        assert_eq!(samples.len(), 2 * segments + 1);
    }

    #[test]
    fn sample_spline_closed_returns_correct_count() {
        let points = vec![
            SplinePoint {
                position: [0.0, 0.0, 0.0],
                tangent_in: [0.0; 3],
                tangent_out: [1.0, 0.0, 0.0],
            },
            SplinePoint {
                position: [3.0, 0.0, 0.0],
                tangent_in: [-1.0, 0.0, 0.0],
                tangent_out: [1.0, 0.0, 0.0],
            },
            SplinePoint {
                position: [6.0, 0.0, 0.0],
                tangent_in: [-1.0, 0.0, 0.0],
                tangent_out: [0.0; 3],
            },
        ];
        let segments = 10;
        let samples = sample_spline(&points, true, segments);
        // 3 curves (closed) * 10 segments + 1 endpoint = 31
        assert_eq!(samples.len(), 3 * segments + 1);
    }

    #[test]
    fn sample_spline_empty() {
        assert!(sample_spline(&[], false, 10).is_empty());
    }

    #[test]
    fn sample_spline_single_point() {
        let points = vec![SplinePoint::default()];
        let samples = sample_spline(&points, false, 10);
        assert_eq!(samples.len(), 1);
    }
}

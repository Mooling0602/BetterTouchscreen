// 几何工具函数 — 纯函数，无副作用，可独立测试

use crate::types::TouchPoint;

/// 计算触控点集合的质心（几何中心）
pub fn centroid(touches: &[TouchPoint]) -> (f64, f64) {
    if touches.is_empty() {
        return (0.0, 0.0);
    }
    let sum_x: f64 = touches.iter().map(|t| t.x).sum();
    let sum_y: f64 = touches.iter().map(|t| t.y).sum();
    let n = touches.len() as f64;
    (sum_x / n, sum_y / n)
}

/// 计算前两个触控点之间的欧几里得距离
pub fn distance(touches: &[TouchPoint]) -> f64 {
    if touches.len() < 2 {
        return 0.0;
    }
    let dx = touches[0].x - touches[1].x;
    let dy = touches[0].y - touches[1].y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tp(x: f64, y: f64) -> TouchPoint {
        TouchPoint {
            x,
            y,
            tracking_id: 0,
        }
    }

    #[test]
    fn centroid_empty() {
        assert_eq!(centroid(&[]), (0.0, 0.0));
    }

    #[test]
    fn centroid_single() {
        assert_eq!(centroid(&[tp(10.0, 20.0)]), (10.0, 20.0));
    }

    #[test]
    fn centroid_two_points() {
        let result = centroid(&[tp(0.0, 0.0), tp(10.0, 10.0)]);
        assert!((result.0 - 5.0).abs() < 1e-10);
        assert!((result.1 - 5.0).abs() < 1e-10);
    }

    #[test]
    fn centroid_symmetric() {
        let result = centroid(&[tp(-5.0, 0.0), tp(5.0, 0.0)]);
        assert!((result.0).abs() < 1e-10);
        assert!((result.1).abs() < 1e-10);
    }

    #[test]
    fn distance_empty() {
        assert_eq!(distance(&[]), 0.0);
    }

    #[test]
    fn distance_single() {
        assert_eq!(distance(&[tp(0.0, 0.0)]), 0.0);
    }

    #[test]
    fn distance_two_points() {
        let result = distance(&[tp(0.0, 0.0), tp(3.0, 4.0)]);
        assert!((result - 5.0).abs() < 1e-10);
    }

    #[test]
    fn distance_same_point() {
        assert_eq!(distance(&[tp(5.0, 5.0), tp(5.0, 5.0)]), 0.0);
    }
}

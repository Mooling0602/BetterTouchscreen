// 公共类型定义 — 触控点、手势事件、手势类型与状态

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub x: f64,
    pub y: f64,
    #[allow(dead_code)]
    pub tracking_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureType {
    Pointer,
    Scroll,
}

#[derive(Debug, Clone)]
pub struct GestureEvent {
    pub gesture_type: GestureType,
    pub fingers: u8,
    pub delta_x: f64,
    pub delta_y: f64,
    pub scale: f64,
    pub state: GestureState,
    /// 手势为静止轻触（2 指无移动 → 右键）
    pub is_tap: bool,
    /// 单指进入拖拽模式（hold → BTN_LEFT 按下）
    pub is_drag: bool,
    /// 手势被中断（手指数变化导致 handler 切换），不应触发点击
    pub suppress_click: bool,
}

impl GestureEvent {
    pub fn new(gesture_type: GestureType, fingers: u8, state: GestureState) -> Self {
        Self {
            gesture_type,
            fingers,
            delta_x: 0.0,
            delta_y: 0.0,
            scale: 1.0,
            state,
            is_tap: false,
            is_drag: false,
            suppress_click: false,
        }
    }

    /// 应用坐标轴变换以补偿屏幕旋转/镜像不匹配
    ///
    /// 变换顺序：先交换 XY，再各自反转，保证变换语义清晰
    pub fn apply_axis_transform(&mut self, swap_axes: bool, invert_x: bool, invert_y: bool) {
        if swap_axes {
            std::mem::swap(&mut self.delta_x, &mut self.delta_y);
        }
        if invert_x {
            self.delta_x = -self.delta_x;
        }
        if invert_y {
            self.delta_y = -self.delta_y;
        }
    }
}

/// 对触控点的绝对坐标应用轴变换（补偿屏幕旋转/镜像）
///
/// `max_x` / `max_y` 应为变换后的有效坐标最大值：
/// 若 `swap_axes` 为 true，调用方应传入交换后的 (max_y, max_x)。
///
/// 与 [`GestureEvent::apply_axis_transform`] 处理增量不同，
/// 此函数对绝对位置进行变换：交换 XY、以及以有效最大值为中轴反射坐标。
pub fn transform_touch_coords(
    touches: &mut [TouchPoint],
    swap_axes: bool,
    invert_x: bool,
    invert_y: bool,
    max_x: f64,
    max_y: f64,
) {
    for point in touches.iter_mut() {
        if swap_axes {
            std::mem::swap(&mut point.x, &mut point.y);
        }
        if invert_x {
            point.x = max_x - point.x;
        }
        if invert_y {
            point.y = max_y - point.y;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureState {
    Begin,
    Update,
    End,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(dx: f64, dy: f64) -> GestureEvent {
        let mut e = GestureEvent::new(GestureType::Pointer, 1, GestureState::Update);
        e.delta_x = dx;
        e.delta_y = dy;
        e
    }

    #[test]
    fn transform_noop() {
        let mut e = make_event(10.0, 20.0);
        e.apply_axis_transform(false, false, false);
        assert!((e.delta_x - 10.0).abs() < 1e-10);
        assert!((e.delta_y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn transform_invert_x() {
        let mut e = make_event(10.0, 20.0);
        e.apply_axis_transform(false, true, false);
        assert!((e.delta_x + 10.0).abs() < 1e-10);
        assert!((e.delta_y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn transform_invert_y() {
        let mut e = make_event(10.0, 20.0);
        e.apply_axis_transform(false, false, true);
        assert!((e.delta_x - 10.0).abs() < 1e-10);
        assert!((e.delta_y + 20.0).abs() < 1e-10);
    }

    #[test]
    fn transform_swap_axes() {
        let mut e = make_event(10.0, 20.0);
        e.apply_axis_transform(true, false, false);
        assert!((e.delta_x - 20.0).abs() < 1e-10);
        assert!((e.delta_y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn transform_180_rotation() {
        // 180°: invert_x=true, invert_y=true
        let mut e = make_event(10.0, -5.0);
        e.apply_axis_transform(false, true, true);
        assert!((e.delta_x + 10.0).abs() < 1e-10);
        assert!((e.delta_y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn transform_90_clockwise() {
        // 90° 顺时针: swap_axes=true, invert_y=true
        let mut e = make_event(10.0, 5.0);
        e.apply_axis_transform(true, false, true);
        assert!((e.delta_x - 5.0).abs() < 1e-10); // was old dy
        assert!((e.delta_y + 10.0).abs() < 1e-10); // -old dx
    }

    #[test]
    fn transform_90_counter_clockwise() {
        // 90° 逆时针: swap_axes=true, invert_x=true
        let mut e = make_event(10.0, 5.0);
        e.apply_axis_transform(true, true, false);
        assert!((e.delta_x + 5.0).abs() < 1e-10); // -old dy
        assert!((e.delta_y - 10.0).abs() < 1e-10); // old dx
    }

    #[test]
    fn transform_all_three() {
        let mut e = make_event(10.0, 5.0);
        e.apply_axis_transform(true, true, true);
        assert!((e.delta_x + 5.0).abs() < 1e-10); // -old dy
        assert!((e.delta_y + 10.0).abs() < 1e-10); // -old dx
    }

    #[test]
    fn transform_does_not_affect_other_fields() {
        let mut e = GestureEvent {
            gesture_type: GestureType::Scroll,
            fingers: 2,
            delta_x: 5.0,
            delta_y: 10.0,
            scale: 1.5,
            state: GestureState::Update,
            is_tap: true,
            is_drag: false,
            suppress_click: true,
        };
        e.apply_axis_transform(false, true, true);
        assert_eq!(e.gesture_type, GestureType::Scroll);
        assert_eq!(e.fingers, 2);
        assert!((e.scale - 1.5).abs() < 1e-10);
        assert!(e.is_tap);
        assert!(!e.is_drag);
        assert!(e.suppress_click);
    }

    fn tp(x: f64, y: f64) -> TouchPoint {
        TouchPoint {
            x,
            y,
            tracking_id: 0,
        }
    }

    #[test]
    fn coord_transform_noop() {
        let mut pts = vec![tp(100.0, 200.0), tp(300.0, 400.0)];
        transform_touch_coords(&mut pts, false, false, false, 1920.0, 1080.0);
        assert!((pts[0].x - 100.0).abs() < 1e-10);
        assert!((pts[0].y - 200.0).abs() < 1e-10);
        assert!((pts[1].x - 300.0).abs() < 1e-10);
        assert!((pts[1].y - 400.0).abs() < 1e-10);
    }

    #[test]
    fn coord_transform_invert_x() {
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, false, true, false, 1920.0, 1080.0);
        assert!((pts[0].x - 1820.0).abs() < 1e-10); // 1920 - 100
        assert!((pts[0].y - 200.0).abs() < 1e-10); // unchanged
    }

    #[test]
    fn coord_transform_invert_y() {
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, false, false, true, 1920.0, 1080.0);
        assert!((pts[0].x - 100.0).abs() < 1e-10); // unchanged
        assert!((pts[0].y - 880.0).abs() < 1e-10); // 1080 - 200
    }

    #[test]
    fn coord_transform_swap() {
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, true, false, false, 1920.0, 1080.0);
        assert!((pts[0].x - 200.0).abs() < 1e-10);
        assert!((pts[0].y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn coord_transform_180_rotation() {
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, false, true, true, 1920.0, 1080.0);
        assert!((pts[0].x - 1820.0).abs() < 1e-10);
        assert!((pts[0].y - 880.0).abs() < 1e-10);
    }

    #[test]
    fn coord_transform_empty() {
        let mut pts: Vec<TouchPoint> = vec![];
        transform_touch_coords(&mut pts, true, true, true, 1920.0, 1080.0);
        assert!(pts.is_empty());
    }

    #[test]
    fn coord_transform_swap_invert_asymmetric() {
        // 不同 max_x/max_y 时，调用方传入有效最大值
        // raw max=(1920,1080), swap=true → eff_max=(1080,1920)
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, true, true, false, 1080.0, 1920.0);
        // 交换: (200, 100) → invert_x 用 eff_max_x=1080: 1080-200=880
        assert!((pts[0].x - 880.0).abs() < 1e-10);
        assert!((pts[0].y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn coord_transform_swap_invert_y_asymmetric() {
        let mut pts = vec![tp(100.0, 200.0)];
        transform_touch_coords(&mut pts, true, false, true, 1080.0, 1920.0);
        // 交换: (200, 100) → invert_y 用 eff_max_y=1920: 1920-100=1820
        assert!((pts[0].x - 200.0).abs() < 1e-10);
        assert!((pts[0].y - 1820.0).abs() < 1e-10);
    }
}

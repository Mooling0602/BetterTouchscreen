// 单指光标处理器 — Pointer：光标移动 + 点击 + 双击按住拖拽

use crate::config::Config;
use crate::gesture::handlers::GestureHandler;
use crate::types::{GestureEvent, GestureState, GestureType, TouchPoint};
use std::ops::RangeInclusive;
use std::time::Instant;

/// 双击最大间隔（ms）：第一次抬起到第二次按下的最大时间
const DOUBLE_TAP_MS: u128 = 300;
/// 双击最大距离（设备像素）：两次触摸位置的最大偏差
const DOUBLE_TAP_RADIUS: f64 = 500.0;

#[derive(Default)]
pub struct PointerHandler {
    sensitivity: f64,
    drag_active: bool,
    has_moved: bool,
    // 上一次轻触记录（用于双击检测）
    last_tap_end: Option<Instant>,
    last_tap_pos: Option<(f64, f64)>,
    touch_start_pos: Option<(f64, f64)>,
}

impl PointerHandler {
    pub fn new(config: &Config) -> Self {
        Self {
            sensitivity: config.pointer_sensitivity,
            ..Default::default()
        }
    }
}

impl GestureHandler for PointerHandler {
    fn finger_range(&self) -> RangeInclusive<u8> {
        1..=1
    }

    fn priority(&self) -> i32 {
        0
    }

    fn name(&self) -> &'static str {
        "Pointer"
    }

    fn begin(&mut self, touches: &[TouchPoint], _prev: &[TouchPoint]) -> Vec<GestureEvent> {
        let start_pos = touches.first().map(|t| (t.x, t.y));

        // 检测双击：上次轻触结束时间在阈值内，且位置足够近
        let is_double_tap = if let (Some(end_time), Some(end_pos), Some(start)) =
            (self.last_tap_end, self.last_tap_pos, start_pos)
        {
            let elapsed = end_time.elapsed().as_millis();
            let dist = ((start.0 - end_pos.0).powi(2) + (start.1 - end_pos.1).powi(2)).sqrt();
            elapsed <= DOUBLE_TAP_MS && dist <= DOUBLE_TAP_RADIUS
        } else {
            false
        };

        self.drag_active = is_double_tap;
        self.has_moved = false;
        self.touch_start_pos = start_pos;
        self.last_tap_end = None;

        let mut event = GestureEvent::new(GestureType::Pointer, 1, GestureState::Begin);
        event.is_drag = is_double_tap;
        vec![event]
    }

    fn update(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent> {
        let (Some(p), Some(c)) = (prev.first(), touches.first()) else {
            return vec![];
        };
        let dx = (c.x - p.x) * self.sensitivity;
        let dy = (c.y - p.y) * self.sensitivity;

        // 追踪是否移动过
        if dx.abs() >= 1.0 || dy.abs() >= 1.0 {
            self.has_moved = true;
        }

        let mut event = GestureEvent::new(GestureType::Pointer, 1, GestureState::Update);
        event.delta_x = dx;
        event.delta_y = dy;
        event.is_drag = self.drag_active;
        vec![event]
    }

    fn end(&mut self, all_fingers_up: bool) -> Vec<GestureEvent> {
        let mut event = GestureEvent::new(GestureType::Pointer, 1, GestureState::End);
        event.is_drag = self.drag_active;
        // 因手指数变化被中断（非所有手指抬起）→ 不触发点击
        event.suppress_click = !all_fingers_up;

        // 只在所有手指离开时记录轻触；因手指数变化切换时清空，防止误判双击
        if all_fingers_up && !self.drag_active && !self.has_moved {
            self.last_tap_end = Some(Instant::now());
            self.last_tap_pos = self.touch_start_pos;
        } else {
            self.last_tap_end = None;
            self.last_tap_pos = None;
        }

        self.drag_active = false;
        self.has_moved = false;
        vec![event]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn tp(x: f64, y: f64) -> TouchPoint {
        TouchPoint {
            x,
            y,
            tracking_id: 0,
        }
    }

    fn handler() -> PointerHandler {
        let mut config = Config::default();
        config.pointer_sensitivity = 1.0; // 测试中不缩放
        PointerHandler::new(&config)
    }

    #[test]
    fn begin_records_start_pos() {
        let mut h = handler();
        let events = h.begin(&[tp(10.0, 20.0)], &[]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].state, GestureState::Begin);
        assert_eq!(events[0].gesture_type, GestureType::Pointer);
        assert!(!events[0].is_drag);
        assert_eq!(h.touch_start_pos, Some((10.0, 20.0)));
        assert!(!h.drag_active);
    }

    #[test]
    fn tap_records_for_double_tap() {
        let mut h = handler();
        // 第一次轻触
        h.begin(&[tp(100.0, 100.0)], &[]);
        h.end(true);
        // 记录了轻触
        assert!(h.last_tap_end.is_some());
        assert_eq!(h.last_tap_pos, Some((100.0, 100.0)));
    }

    #[test]
    fn double_tap_activates_drag() {
        let mut h = handler();
        // 第一次轻触
        h.begin(&[tp(100.0, 100.0)], &[]);
        h.end(true);
        // 第二次按住 → 拖拽
        let events = h.begin(&[tp(102.0, 101.0)], &[]);
        assert!(events[0].is_drag);
        assert!(h.drag_active);
    }

    #[test]
    fn double_tap_too_far_no_drag() {
        let mut h = handler();
        // 第一次轻触
        h.begin(&[tp(100.0, 100.0)], &[]);
        h.end(true);
        // 第二次位置太远 (>500 设备像素) → 不触发拖拽
        let events = h.begin(&[tp(700.0, 700.0)], &[]);
        assert!(!events[0].is_drag);
        assert!(!h.drag_active);
    }

    #[test]
    fn moving_touch_not_recorded_as_tap() {
        let mut h = handler();
        // 第一次触摸并移动
        h.begin(&[tp(100.0, 100.0)], &[]);
        h.update(&[tp(200.0, 200.0)], &[tp(100.0, 100.0)]);
        h.end(true);
        // 有移动 → 不记录轻触
        assert!(h.last_tap_end.is_none());
    }

    #[test]
    fn update_computes_delta() {
        let mut h = handler();
        h.begin(&[tp(10.0, 20.0)], &[]);
        let events = h.update(&[tp(15.0, 25.0)], &[tp(10.0, 20.0)]);
        assert_eq!(events.len(), 1);
        assert!((events[0].delta_x - 5.0).abs() < 1e-10);
        assert!((events[0].delta_y - 5.0).abs() < 1e-10);
        assert_eq!(events[0].state, GestureState::Update);
        assert!(!events[0].is_drag);
    }

    #[test]
    fn update_empty_prev() {
        let mut h = handler();
        let events = h.update(&[tp(10.0, 20.0)], &[]);
        assert!(events.is_empty());
    }

    #[test]
    fn end_returns_end_event() {
        let mut h = handler();
        h.begin(&[tp(10.0, 20.0)], &[]);
        let events = h.end(true);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].state, GestureState::End);
        assert!(!events[0].is_drag);
    }

    #[test]
    fn finger_range_is_1() {
        let h = handler();
        assert_eq!(h.finger_range(), 1..=1);
    }
}

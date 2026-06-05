// 双指滚动处理器 — Scroll

use crate::config::Config;
use crate::gesture::handlers::GestureHandler;
use crate::math::centroid;
use crate::types::{GestureEvent, GestureState, GestureType, TouchPoint};
use log::debug;
use std::ops::RangeInclusive;

/// 预热帧数：Begin 后跳过前几帧的移动检测，避免手指放置时的质心偏移误判
const WARMUP_FRAMES: u8 = 3;

pub struct ScrollHandler {
    config: Config,
    initial_centroid: (f64, f64),
    has_moved: bool,
    warmup_remaining: u8,
}

impl ScrollHandler {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            initial_centroid: (0.0, 0.0),
            has_moved: false,
            warmup_remaining: 0,
        }
    }
}

impl GestureHandler for ScrollHandler {
    fn finger_range(&self) -> RangeInclusive<u8> {
        2..=2
    }

    fn priority(&self) -> i32 {
        0
    }

    fn name(&self) -> &'static str {
        "Scroll"
    }

    fn begin(&mut self, touches: &[TouchPoint], _prev: &[TouchPoint]) -> Vec<GestureEvent> {
        self.initial_centroid = centroid(touches);
        self.has_moved = false;
        self.warmup_remaining = WARMUP_FRAMES;

        debug!("手势开始: Scroll (2 指)");
        vec![GestureEvent::new(GestureType::Scroll, 2, GestureState::Begin)]
    }

    fn update(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent> {
        // 预热期：重新校准初始质心，跳过移动检测
        if self.warmup_remaining > 0 {
            self.warmup_remaining -= 1;
            self.initial_centroid = centroid(touches);
            return vec![];
        }

        let current_centroid = centroid(touches);
        let delta_x = current_centroid.0 - self.initial_centroid.0;
        let delta_y = current_centroid.1 - self.initial_centroid.1;
        let abs_dx = delta_x.abs();
        let abs_dy = delta_y.abs();

        // 未达滚动阈值则静默
        if abs_dx < self.config.scroll_threshold && abs_dy < self.config.scroll_threshold {
            return vec![];
        }

        self.has_moved = true;

        // 帧间增量（应用灵敏度）
        let prev_centroid = centroid(prev);
        let frame_dx = (current_centroid.0 - prev_centroid.0) * self.config.scroll_sensitivity;
        let frame_dy = (current_centroid.1 - prev_centroid.1) * self.config.scroll_sensitivity;

        let mut event = GestureEvent::new(GestureType::Scroll, 2, GestureState::Update);
        event.delta_x = frame_dx;
        event.delta_y = frame_dy;
        vec![event]
    }

    fn end(&mut self, _all_fingers_up: bool) -> Vec<GestureEvent> {
        if self.has_moved {
            debug!("手势结束: Scroll");
            vec![GestureEvent::new(GestureType::Scroll, 2, GestureState::End)]
        } else {
            debug!("手势结束: Scroll → 2 指轻触 (右键)");
            let mut event = GestureEvent::new(GestureType::Scroll, 2, GestureState::End);
            event.is_tap = true;
            vec![event]
        }
    }
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

    fn two_finger(cx: f64, cy: f64, gap: f64) -> Vec<TouchPoint> {
        vec![tp(cx - gap / 2.0, cy), tp(cx + gap / 2.0, cy)]
    }

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn begin_records_initial_state() {
        let mut h = ScrollHandler::new(default_config());
        let touches = two_finger(100.0, 100.0, 50.0);
        let events = h.begin(&touches, &[]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].state, GestureState::Begin);
        assert_eq!(events[0].gesture_type, GestureType::Scroll);
    }

    #[test]
    fn update_below_threshold_silent() {
        let mut h = ScrollHandler::new(default_config());
        let t0 = two_finger(100.0, 100.0, 50.0);
        h.begin(&t0, &[]);
        // 移动 2px < scroll_threshold(5.0)
        let t1 = two_finger(102.0, 100.0, 50.0);
        let events = h.update(&t1, &t0);
        assert!(events.is_empty());
    }

    #[test]
    fn update_above_threshold_emits_event() {
        let mut h = ScrollHandler::new(default_config());
        let t0 = two_finger(100.0, 100.0, 50.0);
        h.begin(&t0, &[]);
        // 消耗预热帧
        for _ in 0..WARMUP_FRAMES {
            h.update(&t0, &t0);
        }
        let t1 = two_finger(110.0, 100.0, 50.0);
        let events = h.update(&t1, &t0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].gesture_type, GestureType::Scroll);
    }

    #[test]
    fn end_returns_end_event() {
        let mut h = ScrollHandler::new(default_config());
        let events = h.end(true);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].state, GestureState::End);
    }
}

// 手势引擎端到端集成测试

use bettertouchscreen::config::Config;
use bettertouchscreen::gesture::engine::GestureEngine;
use bettertouchscreen::types::{GestureState, GestureType, TouchPoint};

fn tp(x: f64, y: f64) -> TouchPoint {
    TouchPoint {
        x,
        y,
        tracking_id: 0,
    }
}

fn engine() -> GestureEngine {
    GestureEngine::new(Config::default())
}

// === 单指 Pointer ===

#[test]
fn pointer_single_finger_tap() {
    let mut e = engine();

    // 手指按下
    let events = e.process(&[tp(100.0, 100.0)]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Pointer);
    assert_eq!(events[0].state, GestureState::Begin);

    // 手指抬起
    let events = e.process(&[]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, GestureState::End);
}

#[test]
fn pointer_single_finger_drag() {
    let mut e = engine();

    // 第一次轻触（短按后抬起）
    e.process(&[tp(100.0, 100.0)]); // Begin
    e.process(&[]); // End（轻触）

    // 第二次按住 → 拖拽待确认
    let events = e.process(&[tp(102.0, 101.0)]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Pointer);
    assert!(!events[0].is_drag); // 需保持足够时长才激活拖拽

    // 等待保持时间后 update → 拖拽激活
    std::thread::sleep(std::time::Duration::from_millis(90));
    let events = e.process(&[tp(102.0, 101.0)]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, GestureState::Update);
    assert!(events[0].is_drag); // 拖拽已确认

    // 移动（灵敏度 0.5：dx=10*0.5=5, dy=5*0.5=2.5）
    let events = e.process(&[tp(112.0, 106.0)]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, GestureState::Update);
    assert!(events[0].is_drag);
    assert!((events[0].delta_x - 5.0).abs() < 1e-10);
    assert!((events[0].delta_y - 2.5).abs() < 1e-10);
}

// === 双指 Scroll ===

#[test]
fn scroll_two_finger() {
    let mut e = engine();
    let t0 = vec![tp(80.0, 100.0), tp(120.0, 100.0)];

    // Begin
    let events = e.process(&t0);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Scroll);
    assert_eq!(events[0].state, GestureState::Begin);

    // 消耗预热帧（3帧）
    for _ in 0..3 {
        e.process(&t0);
    }

    // 大幅度平移 → 触发滚动
    let t1 = vec![tp(90.0, 100.0), tp(130.0, 100.0)];
    let events = e.process(&t1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Scroll);
    assert_eq!(events[0].state, GestureState::Update);

    // End
    let events = e.process(&[]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, GestureState::End);
}

// === 手指数量切换 ===

#[test]
fn finger_count_change_switches_handler() {
    let mut e = engine();

    // 单指开始
    let events = e.process(&[tp(100.0, 100.0)]);
    assert_eq!(events[0].gesture_type, GestureType::Pointer);

    // 加到双指 → 切换到 Scroll
    let t2 = vec![tp(80.0, 100.0), tp(120.0, 100.0)];
    let events = e.process(&t2);
    // 应有 Pointer End + Scroll Begin
    let has_pointer_end = events
        .iter()
        .any(|e| e.gesture_type == GestureType::Pointer && e.state == GestureState::End);
    let has_scroll_begin = events
        .iter()
        .any(|e| e.gesture_type == GestureType::Scroll && e.state == GestureState::Begin);
    assert!(has_pointer_end);
    assert!(has_scroll_begin);
}

// === 空输入 ===

#[test]
fn empty_input_clears_state() {
    let mut e = engine();
    e.process(&[tp(100.0, 100.0)]); // Pointer Begin

    let events = e.process(&[]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, GestureState::End);

    // 再次空输入 → 无事件
    let events = e.process(&[]);
    assert!(events.is_empty());
}

// === 坐标轴变换 ===

fn engine_with_transform(swap_axes: bool, invert_x: bool, invert_y: bool) -> GestureEngine {
    let mut config = Config::default();
    config.swap_axes = swap_axes;
    config.invert_x = invert_x;
    config.invert_y = invert_y;
    GestureEngine::new(config)
}

#[test]
fn axis_transform_invert_x_pointer() {
    let mut e = engine_with_transform(false, true, false);
    // 单指向右移动
    e.process(&[tp(100.0, 100.0)]); // Begin
    let events = e.process(&[tp(120.0, 100.0)]); // Update
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Pointer);
    // invert_x: dx 应为负值（实际 dx=20*sensitivity=10，取反 = -10）
    assert!(events[0].delta_x < -1.0);
    assert!(events[0].delta_y.abs() < 0.1); // dy 接近 0
}

#[test]
fn axis_transform_swap_axes_pointer() {
    let mut e = engine_with_transform(true, false, false);
    // 单指 Y 方向移动
    e.process(&[tp(100.0, 100.0)]); // Begin
    let events = e.process(&[tp(100.0, 130.0)]); // Update
    assert_eq!(events.len(), 1);
    // swap_axes: dy 变为 dx（delta_x 应为正，delta_y 接近 0）
    assert!(events[0].delta_x.abs() > 1.0);
    assert!(events[0].delta_y.abs() < 0.1);
}

#[test]
fn axis_transform_invert_y_scroll() {
    let mut e = engine_with_transform(false, false, true);
    let t0 = vec![tp(80.0, 100.0), tp(120.0, 100.0)];

    e.process(&t0); // Begin
    // 消耗预热帧
    for _ in 0..3 {
        e.process(&t0);
    }

    // 向下移动（delta_y 为正）
    let t1 = vec![tp(80.0, 130.0), tp(120.0, 130.0)];
    let events = e.process(&t1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].gesture_type, GestureType::Scroll);
    // invert_y: dy 应为负值（方向反转）
    assert!(events[0].delta_y < 0.0);
}

#[test]
fn axis_transform_180_rotation() {
    // 180° 旋转: invert_x + invert_y
    let mut e = engine_with_transform(false, true, true);
    e.process(&[tp(100.0, 100.0)]);
    let events = e.process(&[tp(130.0, 120.0)]);
    assert_eq!(events.len(), 1);
    // 两个轴都反转：dx 和 dy 都应为负
    assert!(events[0].delta_x < -1.0);
    assert!(events[0].delta_y < -1.0);
}

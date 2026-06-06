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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureState {
    Begin,
    Update,
    End,
}

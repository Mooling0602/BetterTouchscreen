// 手势处理器 trait

pub mod pointer;
pub mod scroll;

use crate::types::{GestureEvent, TouchPoint};
use std::ops::RangeInclusive;

/// 每个手势类型实现此 trait，封装自身的检测、状态管理和事件生成逻辑
pub trait GestureHandler: Send {
    /// 此 handler 处理的手指数范围
    fn finger_range(&self) -> RangeInclusive<u8>;

    /// 优先级，值越大越优先匹配。默认 0
    fn priority(&self) -> i32 {
        0
    }

    /// handler 名称，用于日志
    fn name(&self) -> &'static str;

    /// 手势被激活时调用，返回 Begin 事件
    fn begin(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent>;

    /// 手势持续中每帧调用，返回 Update 事件（可能为空）
    fn update(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent>;

    /// 手势结束时调用，返回 End 事件
    /// all_fingers_up: true = 所有手指离开屏幕，false = 因手指数变化而切换
    fn end(&mut self, all_fingers_up: bool) -> Vec<GestureEvent>;
}

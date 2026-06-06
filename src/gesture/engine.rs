// 手势引擎 — 纯调度器，根据手指数路由到对应 handler

use crate::config::Config;
use crate::gesture::handlers::GestureHandler;
use crate::gesture::handlers::pointer::PointerHandler;
use crate::gesture::handlers::scroll::ScrollHandler;
use crate::types::{GestureEvent, TouchPoint};
use log::{debug, trace};
use std::ops::RangeInclusive;

/// 静态分发的 gesture handler 包装枚举
enum GestureHandlerWrapper {
    Pointer(PointerHandler),
    Scroll(ScrollHandler),
}

impl GestureHandlerWrapper {
    fn finger_range(&self) -> RangeInclusive<u8> {
        match self {
            Self::Pointer(h) => h.finger_range(),
            Self::Scroll(h) => h.finger_range(),
        }
    }

    fn priority(&self) -> i32 {
        match self {
            Self::Pointer(h) => h.priority(),
            Self::Scroll(h) => h.priority(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Pointer(h) => h.name(),
            Self::Scroll(h) => h.name(),
        }
    }

    fn begin(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent> {
        match self {
            Self::Pointer(h) => h.begin(touches, prev),
            Self::Scroll(h) => h.begin(touches, prev),
        }
    }

    fn update(&mut self, touches: &[TouchPoint], prev: &[TouchPoint]) -> Vec<GestureEvent> {
        match self {
            Self::Pointer(h) => h.update(touches, prev),
            Self::Scroll(h) => h.update(touches, prev),
        }
    }

    fn end(&mut self, all_fingers_up: bool) -> Vec<GestureEvent> {
        match self {
            Self::Pointer(h) => h.end(all_fingers_up),
            Self::Scroll(h) => h.end(all_fingers_up),
        }
    }
}

pub struct GestureEngine {
    handlers: Vec<GestureHandlerWrapper>,
    active: Option<usize>,
    prev_touches: Vec<TouchPoint>,
    /// 多指手势结束后有残留手指时置 true，防止残留手指触发新的 Pointer 点击
    was_multitouch: bool,
    /// 触控坐标轴变换配置（屏幕旋转补偿）
    config: Config,
}

impl GestureEngine {
    pub fn new(config: Config) -> Self {
        let handlers = vec![
            GestureHandlerWrapper::Pointer(PointerHandler::new(&config)),
            GestureHandlerWrapper::Scroll(ScrollHandler::new(config)),
        ];

        Self {
            handlers,
            active: None,
            prev_touches: Vec::new(),
            was_multitouch: false,
            config,
        }
    }

    pub fn process(&mut self, touches: &[TouchPoint]) -> Vec<GestureEvent> {
        let finger_count = touches.len() as u8;
        trace!(
            "引擎 process: {} 指, active={:?}",
            finger_count, self.active
        );

        // 1. 所有手指离开 → 结束当前手势
        if finger_count == 0 {
            let mut events = self.deactivate(true);
            self.prev_touches.clear();
            self.was_multitouch = false;
            self.apply_config_transform(&mut events);
            return events;
        }

        // 2. 多指手势结束后仍有残留手指 → 压制，直到所有手指抬起
        if self.was_multitouch {
            self.prev_touches = touches.to_vec();
            return vec![];
        }

        // 3. 检查当前 handler 是否仍匹配 finger_count
        if let Some(idx) = self.active
            && !self.handlers[idx].finger_range().contains(&finger_count)
        {
            debug!(
                "手指数变化，切换 handler: {:?} → {}指",
                self.active, finger_count
            );
            let handler_min = *self.handlers[idx].finger_range().start();
            let was_multifinger = handler_min >= 2;
            let mut events = self.deactivate(false);
            // 仅当手指数减少到低于 handler 最小手指数时压制（如 2→1），
            // 手指数增加时（如 2→3）不压制，允许上层 re-activate
            if was_multifinger && finger_count > 0 && finger_count < handler_min {
                self.was_multitouch = true;
                self.prev_touches = touches.to_vec();
                self.apply_config_transform(&mut events);
                return events;
            }
            events.extend(self.try_activate(finger_count, touches));
            self.prev_touches = touches.to_vec();
            self.apply_config_transform(&mut events);
            return events;
        }

        // 5. 无激活 handler → 尝试激活
        if self.active.is_none() {
            let mut events = self.try_activate(finger_count, touches);
            self.prev_touches = touches.to_vec();
            self.apply_config_transform(&mut events);
            return events;
        }

        // 6. 正常更新
        let mut events = self.call_update(touches);
        trace!("引擎 update 产生 {} 个事件", events.len());

        self.prev_touches = touches.to_vec();
        self.apply_config_transform(&mut events);
        events
    }

    fn apply_config_transform(&self, events: &mut [GestureEvent]) {
        if self.config.swap_axes || self.config.invert_x || self.config.invert_y {
            for event in events.iter_mut() {
                event.apply_axis_transform(
                    self.config.swap_axes,
                    self.config.invert_x,
                    self.config.invert_y,
                );
            }
        }
    }

    fn call_update(&mut self, touches: &[TouchPoint]) -> Vec<GestureEvent> {
        if let Some(idx) = self.active {
            let prev = self.prev_touches.clone();
            self.handlers[idx].update(touches, &prev)
        } else {
            vec![]
        }
    }

    fn try_activate(&mut self, finger_count: u8, touches: &[TouchPoint]) -> Vec<GestureEvent> {
        let best = self
            .handlers
            .iter()
            .enumerate()
            .filter(|(_, h)| h.finger_range().contains(&finger_count))
            .max_by_key(|(_, h)| h.priority());

        if let Some((idx, handler)) = best {
            debug!("激活 handler: {} ({} 指)", handler.name(), finger_count);
            self.active = Some(idx);
            let prev = self.prev_touches.clone();
            self.handlers[idx].begin(touches, &prev)
        } else {
            vec![]
        }
    }

    fn deactivate(&mut self, all_fingers_up: bool) -> Vec<GestureEvent> {
        match self.active.take() {
            Some(idx) => self.handlers[idx].end(all_fingers_up),
            None => vec![],
        }
    }
}

impl Default for GestureEngine {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

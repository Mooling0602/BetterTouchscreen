// 手势引擎 — 纯调度器，根据手指数路由到对应 handler

use crate::config::Config;
use crate::gesture::handlers::pointer::PointerHandler;
use crate::gesture::handlers::scroll::ScrollHandler;
use crate::gesture::handlers::GestureHandler;
use crate::types::{GestureEvent, TouchPoint};
use log::{debug, trace};

pub struct GestureEngine {
    handlers: Vec<Box<dyn GestureHandler>>,
    active: Option<usize>,
    prev_touches: Vec<TouchPoint>,
}

impl GestureEngine {
    pub fn new(config: Config) -> Self {
        let handlers: Vec<Box<dyn GestureHandler>> = vec![
            Box::new(PointerHandler::new(&config)),
            Box::new(ScrollHandler::new(config)),
        ];

        Self {
            handlers,
            active: None,
            prev_touches: Vec::new(),
        }
    }

    pub fn process(&mut self, touches: &[TouchPoint]) -> Vec<GestureEvent> {
        let finger_count = touches.len() as u8;
        trace!("引擎 process: {} 指, active={:?}", finger_count, self.active);

        // 1. 所有手指离开 → 结束当前手势
        if finger_count == 0 {
            let events = self.deactivate(true);
            self.prev_touches.clear();
            return events;
        }

        // 2. 检查当前 handler 是否仍匹配 finger_count
        if let Some(idx) = self.active
            && !self.handlers[idx].finger_range().contains(&finger_count)
        {
            debug!("手指数变化，切换 handler: {:?} → {}指", self.active, finger_count);
            let mut events = self.deactivate(false);
            events.extend(self.try_activate(finger_count, touches));
            self.prev_touches = touches.to_vec();
            return events;
        }

        // 3. 无激活 handler → 尝试激活
        if self.active.is_none() {
            let events = self.try_activate(finger_count, touches);
            self.prev_touches = touches.to_vec();
            return events;
        }

        // 4. 正常更新
        let events = self.call_update(touches);
        trace!("引擎 update 产生 {} 个事件", events.len());

        self.prev_touches = touches.to_vec();
        events
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

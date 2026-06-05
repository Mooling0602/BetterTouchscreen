// 低级 evdev 事件构建辅助 — 封装 uinput 设备 emit 操作

use anyhow::Result;
use evdev::uinput::VirtualDevice;
use evdev::{InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode};

pub struct EventEmitter<'a> {
    device: &'a mut VirtualDevice,
}

impl<'a> EventEmitter<'a> {
    pub fn new(device: &'a mut VirtualDevice) -> Self {
        Self { device }
    }

    pub fn emit_scroll(&mut self, dx: f64, dy: f64) -> Result<()> {
        let h_scroll = dx as i32;
        let v_scroll = dy as i32;

        let mut events: Vec<InputEvent> = Vec::new();

        if h_scroll != 0 {
            events.push(evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_HWHEEL, h_scroll).into());
        }

        if v_scroll != 0 {
            events.push(evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_WHEEL, v_scroll).into());
        }

        if !events.is_empty() {
            events
                .push(evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into());
            self.device.emit(&events)?;
        }
        Ok(())
    }

    pub fn emit_pointer_move(&mut self, dx: f64, dy: f64) -> Result<()> {
        let mut events: Vec<InputEvent> = Vec::new();

        if dx.abs() >= 1.0 {
            events.push(evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_X, dx as i32).into());
        }
        if dy.abs() >= 1.0 {
            events.push(evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_Y, dy as i32).into());
        }

        if !events.is_empty() {
            events
                .push(evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into());
            self.device.emit(&events)?;
        }
        Ok(())
    }

    pub fn emit_click(&mut self) -> Result<()> {
        self.emit_click_key(KeyCode::BTN_LEFT)
    }

    pub fn emit_right_click(&mut self) -> Result<()> {
        self.emit_click_key(KeyCode::BTN_RIGHT)
    }

    fn emit_click_key(&mut self, key: KeyCode) -> Result<()> {
        let events: Vec<InputEvent> = vec![
            evdev::KeyEvent::new(key, 1).into(),
            evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
        ];
        self.device.emit(&events)?;

        let release: Vec<InputEvent> = vec![
            evdev::KeyEvent::new(key, 0).into(),
            evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
        ];
        self.device.emit(&release)?;
        Ok(())
    }

    pub fn emit_button(&mut self, key: KeyCode, press: bool) -> Result<()> {
        let value = if press { 1 } else { 0 };
        let events: Vec<InputEvent> = vec![
            evdev::KeyEvent::new(key, value).into(),
            evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
        ];
        self.device.emit(&events)?;
        Ok(())
    }
}

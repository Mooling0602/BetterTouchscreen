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

    pub fn emit_scroll(&mut self, dx: f64, dy: f64, hscroll_threshold: i32) -> Result<()> {
        let h_scroll = dx as i32;
        let v_scroll = dy as i32;
        let has_h = hscroll_threshold > 0 && h_scroll.abs() >= hscroll_threshold;

        // 无有效水平滚动 → 纯垂直滚动
        if !has_h && v_scroll != 0 {
            let events: Vec<InputEvent> = vec![
                evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_WHEEL, v_scroll).into(),
                evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
            ];
            self.device.emit(&events)?;
            return Ok(());
        }

        // 无有效滚动
        if !has_h && v_scroll == 0 {
            return Ok(());
        }

        // 水平滚动：模拟 Shift + 滚轮
        if has_h {
            let shift_press = vec![
                evdev::KeyEvent::new(KeyCode::KEY_LEFTSHIFT, 1).into(),
                evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
            ];
            self.device.emit(&shift_press)?;

            let wheel = vec![
                evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_WHEEL, h_scroll).into(),
                evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
            ];
            self.device.emit(&wheel)?;

            // 如果同时有垂直滚动，在同一次 Shift 按下中发出
            if v_scroll != 0 {
                let v_wheel = vec![
                    evdev::RelativeAxisEvent::new(RelativeAxisCode::REL_WHEEL, v_scroll).into(),
                    evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
                ];
                self.device.emit(&v_wheel)?;
            }

            let shift_release = vec![
                evdev::KeyEvent::new(KeyCode::KEY_LEFTSHIFT, 0).into(),
                evdev::SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into(),
            ];
            self.device.emit(&shift_release)?;
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

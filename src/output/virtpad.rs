// 虚拟触摸板 — 通过 uinput 创建虚拟设备，发射手势事件
//
// 输出 REL_X/REL_Y 相对坐标事件（与 TouchpadEmulator 相同策略），
// 设置 INPUT_PROP_DIRECT 属性让 libinput 正确识别并应用指针加速。

use crate::output::emitter::EventEmitter;
use crate::types::{GestureEvent, GestureState, GestureType};
use anyhow::{Context, Result};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, KeyCode, PropType, RelativeAxisCode};
use log::{debug, info, trace};
use std::time::Instant;

/// 拖拽结束后延迟释放 BTN_LEFT 的窗口（ms），允许紧随的右键保持选区
const DRAG_RELEASE_WINDOW_MS: u128 = 150;

pub struct VirtualTouchpad {
    device: VirtualDevice,
    pointer_active: bool,
    pointer_moved: bool,
    drag_active: bool,
    // 亚像素累积（防止逐帧截断丢失精度）
    subpixel_x: f64,
    subpixel_y: f64,
    // 滚动亚像素累积
    scroll_sub_y: f64,
    /// 拖拽结束后延迟释放 BTN_LEFT 的截止时间
    drag_release_deadline: Option<Instant>,
}

impl VirtualTouchpad {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);
        rel_axes.insert(RelativeAxisCode::REL_WHEEL);

        // INPUT_PROP_DIRECT — 与 TouchpadEmulator 一致，让 libinput 正确识别
        let mut props = AttributeSet::<PropType>::new();
        props.insert(PropType(0x01)); // INPUT_PROP_DIRECT

        let device = VirtualDevice::builder()?
            .name("BetterTouchscreen Virtual Touchpad")
            .with_keys(&keys)?
            .with_relative_axes(&rel_axes)?
            .with_properties(&props)?
            .build()
            .context("无法创建 uinput 虚拟触摸板设备")?;

        info!("已创建 uinput 虚拟触摸板设备 (REL, INPUT_PROP_DIRECT)");
        Ok(Self {
            device,
            pointer_active: false,
            pointer_moved: false,
            drag_active: false,
            subpixel_x: 0.0,
            subpixel_y: 0.0,
            scroll_sub_y: 0.0,
            drag_release_deadline: None,
        })
    }

    pub fn emit_gesture(&mut self, event: &GestureEvent) -> Result<()> {
        // 检查拖拽延迟释放是否超时
        if let Some(deadline) = self.drag_release_deadline
            && Instant::now() >= deadline
        {
            release_drag(&mut self.device)?;
            self.drag_active = false;
            self.drag_release_deadline = None;
        }

        trace!(
            "输出手势: {:?} state={:?} fingers={} delta=({:.1},{:.1}) scale={:.3} tap={} drag={}",
            event.gesture_type,
            event.state,
            event.fingers,
            event.delta_x,
            event.delta_y,
            event.scale,
            event.is_tap,
            event.is_drag
        );
        match event.state {
            GestureState::Begin => {
                // 新手势开始时，如果拖拽已结束，释放 BTN_LEFT
                if self.drag_release_deadline.is_some() {
                    release_drag(&mut self.device)?;
                    self.drag_active = false;
                    self.drag_release_deadline = None;
                }
                debug!(
                    "输出手势开始: {:?} ({}指)",
                    event.gesture_type, event.fingers
                );
                if event.gesture_type == GestureType::Pointer {
                    self.pointer_active = true;
                    self.pointer_moved = false;
                    if event.is_drag {
                        // 双击按住 → 立即按下 BTN_LEFT
                        self.drag_active = true;
                        let mut emitter = EventEmitter::new(&mut self.device);
                        emitter.emit_button(KeyCode::BTN_LEFT, true)?;
                        debug!("拖拽开始: BTN_LEFT 按下");
                    } else {
                        self.drag_active = false;
                    }
                    self.subpixel_x = 0.0;
                    self.subpixel_y = 0.0;
                } else if event.gesture_type == GestureType::Scroll {
                    self.scroll_sub_y = 0.0;
                }
            }
            GestureState::End => {
                self.handle_gesture_end(event)?;
            }
            GestureState::Update => {
                self.handle_gesture_update(event)?;
            }
        }
        Ok(())
    }

    fn handle_gesture_update(&mut self, event: &GestureEvent) -> Result<()> {
        // 实际滚动开始时，如果拖拽 BTN_LEFT 还按着，立即释放
        if event.gesture_type == GestureType::Scroll && self.drag_release_deadline.is_some() {
            release_drag(&mut self.device)?;
            self.drag_active = false;
            self.drag_release_deadline = None;
        }

        let mut emitter = EventEmitter::new(&mut self.device);
        match event.gesture_type {
            GestureType::Pointer => {
                if event.is_drag && !self.drag_active {
                    // 进入拖拽模式：按下 BTN_LEFT
                    self.drag_active = true;
                    emitter.emit_button(KeyCode::BTN_LEFT, true)?;
                    debug!("拖拽开始: BTN_LEFT 按下");
                }
                self.pointer_moved = true;
                // 亚像素累积：保留小数精度
                self.subpixel_x += event.delta_x;
                self.subpixel_y += event.delta_y;
                let int_x = self.subpixel_x as i32;
                let int_y = self.subpixel_y as i32;
                self.subpixel_x -= int_x as f64;
                self.subpixel_y -= int_y as f64;
                if int_x != 0 || int_y != 0 {
                    emitter.emit_pointer_move(int_x as f64, int_y as f64)?;
                }
            }
            GestureType::Scroll => {
                // 亚像素累积 + clamp 防止偶发大跳变
                self.scroll_sub_y += event.delta_y.clamp(-5.0, 5.0);
                let int_y = self.scroll_sub_y as i32;
                self.scroll_sub_y -= int_y as f64;
                if int_y != 0 {
                    emitter.emit_scroll(int_y)?;
                }
            }
        }
        Ok(())
    }

    fn handle_gesture_end(&mut self, event: &GestureEvent) -> Result<()> {
        match event.gesture_type {
            GestureType::Pointer => {
                if self.drag_active {
                    // 延迟释放 BTN_LEFT，保留选区供后续右键使用
                    self.drag_release_deadline = Some(
                        Instant::now()
                            + std::time::Duration::from_millis(DRAG_RELEASE_WINDOW_MS as u64),
                    );
                    debug!("拖拽结束: 延迟释放 BTN_LEFT ({}ms)", DRAG_RELEASE_WINDOW_MS);
                } else if !event.suppress_click && !self.pointer_moved {
                    // 无移动的轻触 → 左键点击（被中断的 handler 切换除外）
                    let mut emitter = EventEmitter::new(&mut self.device);
                    emitter.emit_click()?;
                }
                self.pointer_active = false;
                self.pointer_moved = false;
            }
            GestureType::Scroll if event.is_tap => {
                // 2 指无移动 → 右键点击
                if self.drag_release_deadline.is_some() {
                    release_drag(&mut self.device)?;
                    self.drag_active = false;
                    self.drag_release_deadline = None;
                }
                debug!("2 指轻触 → 右键");
                let mut emitter = EventEmitter::new(&mut self.device);
                emitter.emit_right_click()?;
                self.scroll_sub_y = 0.0;
            }
            GestureType::Scroll => {
                self.scroll_sub_y = 0.0;
            }
        }
        debug!("输出手势结束: {:?}", event.gesture_type);
        Ok(())
    }
}

fn release_drag(device: &mut VirtualDevice) -> Result<()> {
    let events: Vec<evdev::InputEvent> = vec![
        evdev::KeyEvent::new(KeyCode::BTN_LEFT, 0).into(),
        evdev::SynchronizationEvent::new(evdev::SynchronizationCode::SYN_REPORT, 0).into(),
    ];
    device.emit(&events)?;
    debug!("拖拽结束: BTN_LEFT 释放");
    Ok(())
}

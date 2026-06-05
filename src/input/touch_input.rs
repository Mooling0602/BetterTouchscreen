use crate::types::TouchPoint;
use anyhow::{Context, Result};
use evdev::{AbsoluteAxisCode, BusType, Device, EventSummary, SynchronizationCode};
use log::{debug, info, trace, warn};
use std::fs::OpenOptions;
use std::os::fd::OwnedFd;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Instant;

const MAX_SLOTS: usize = 16;
/// 陈旧 slot 清除阈值：tracking_id=-1 时，清理超过此时间未更新的 slot
const STALE_SLOT_MS: u128 = 100;

#[derive(Debug, Clone, Copy)]
struct SlotInfo {
    tracking_id: i32,
    x: i32,
    y: i32,
    /// 首次 X 和 Y 都收到后才为 true，防止初始 0 值导致光标跳变
    initialized: bool,
    /// 最后一次位置更新时间，用于检测陈旧 slot
    last_update: Instant,
}

pub struct TouchScreen {
    device: Device,
    slots: [Option<SlotInfo>; MAX_SLOTS],
    current_slot: usize,
}

pub fn list_touchscreens() -> Vec<(String, String, bool)> {
    let mut devices = Vec::new();
    for (path, device) in evdev::enumerate() {
        if device
            .supported_absolute_axes()
            .is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X))
        {
            let name = device.name().unwrap_or("未知");
            let name_lower = name.to_lowercase();
            // 仅排除触摸板，保留虚拟触屏（如 Sunshine/Apollo 串流设备）
            if name_lower.contains("touchpad") || name_lower.contains("trackpad") {
                continue;
            }

            let bus = device.input_id().bus_type();
            let is_direct =
                bus == BusType::BUS_I2C || bus == BusType::BUS_USB || bus == BusType::BUS_VIRTUAL;

            devices.push((
                path.to_string_lossy().to_string(),
                device.name().unwrap_or("未知").to_string(),
                is_direct,
            ));
        }
    }
    devices
}

impl TouchScreen {
    pub fn open(device_path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(device_path)
            .with_context(|| format!("无法打开触屏设备: {}", device_path))?;

        let fd = OwnedFd::from(file);
        let mut device = Device::from_fd(fd)
            .with_context(|| format!("无法从 fd 创建 Device: {}", device_path))?;

        match device.grab() {
            Ok(()) => info!("已获取设备独占权 (EVIOCGRAB)"),
            Err(e) => warn!("无法获取设备独占权: {}（Compositor 可能已占用）", e),
        }

        info!(
            "已打开触屏设备: {} ({})",
            device_path,
            device.name().unwrap_or("未知")
        );
        Ok(Self {
            device,
            slots: [None; MAX_SLOTS],
            current_slot: 0,
        })
    }

    pub fn find_touchscreen() -> Result<Self> {
        let candidates = list_touchscreens();

        if candidates.is_empty() {
            anyhow::bail!(
                "未找到触屏设备，请通过 --device 参数指定设备路径，或使用 --list 查看所有输入设备"
            );
        }

        // 优先选择总线连接的真实触屏设备（非虚拟/非触摸板）
        let real_candidates: Vec<_> = candidates
            .iter()
            .filter(|(_, _, is_direct)| *is_direct)
            .collect();

        let chosen = if !real_candidates.is_empty() {
            info!(
                "从 {} 个多点触控设备中判定 {} 个为真实触屏，选择:",
                candidates.len(),
                real_candidates.len()
            );
            for (i, (path, name, _)) in real_candidates.iter().enumerate() {
                info!("  [{i}] {} ({})", path, name);
            }
            real_candidates[0]
        } else if !candidates.is_empty() {
            info!("自动检测到多点触控设备（无物理总线连接触屏）:");
            for (i, (path, name, _)) in candidates.iter().enumerate() {
                info!("  [{i}] {} ({})", path, name);
            }
            info!("提示: 使用 --device <路径> 可选择指定设备");
            &candidates[0]
        } else {
            anyhow::bail!(
                "未找到多点触控设备，请使用 --list 查看所有设备，或通过 --device 指定路径"
            );
        };

        Self::open(&chosen.0)
    }

    pub fn name(&self) -> Option<&str> {
        self.device.name()
    }

    pub fn read_touch_frame(&mut self) -> Result<Vec<TouchPoint>> {
        loop {
            let events: Vec<_> = match self.device.fetch_events() {
                Ok(iter) => iter.collect(),
                Err(e) if e.raw_os_error() == Some(libc::EAGAIN) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            if events.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            for ev in events {
                trace!("evdev: {:?}", ev);
                match ev.destructure() {
                    EventSummary::AbsoluteAxis(_, axis, value) => match axis {
                        AbsoluteAxisCode::ABS_MT_SLOT => {
                            self.current_slot = value as usize;
                            if self.current_slot >= MAX_SLOTS {
                                warn!(
                                    "触控点槽位 {} 超出最大支持范围 {}",
                                    self.current_slot, MAX_SLOTS
                                );
                            }
                        }
                        AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
                            if self.current_slot < MAX_SLOTS {
                                if value == -1 {
                                    self.slots[self.current_slot] = None;
                                    // 清理陈旧 slot：驱动可能漏发 tracking_id=-1
                                    let now = Instant::now();
                                    for slot in self.slots.iter_mut() {
                                        if let Some(info) = slot
                                            && now.duration_since(info.last_update).as_millis() > STALE_SLOT_MS
                                        {
                                            trace!("清除陈旧 slot: tracking_id={}", info.tracking_id);
                                            *slot = None;
                                        }
                                    }
                                } else {
                                    // 清理同一 tracking_id 在其他槽位的旧数据
                                    for slot in self.slots.iter_mut() {
                                        if let Some(info) = slot
                                            && info.tracking_id == value
                                        {
                                            *slot = None;
                                        }
                                    }
                                    self.slots[self.current_slot] = Some(SlotInfo {
                                        tracking_id: value,
                                        x: 0,
                                        y: 0,
                                        initialized: false,
                                        last_update: Instant::now(),
                                    });
                                }
                            }
                        }
                        AbsoluteAxisCode::ABS_MT_POSITION_X => {
                            if self.current_slot < MAX_SLOTS
                                && let Some(info) = &mut self.slots[self.current_slot]
                            {
                                info.x = value;
                                info.last_update = Instant::now();
                                if !info.initialized && info.y != 0 {
                                    info.initialized = true;
                                }
                            }
                        }
                        AbsoluteAxisCode::ABS_MT_POSITION_Y if self.current_slot < MAX_SLOTS => {
                            if let Some(info) = &mut self.slots[self.current_slot] {
                                info.y = value;
                                info.last_update = Instant::now();
                                if !info.initialized && info.x != 0 {
                                    info.initialized = true;
                                }
                            }
                        }
                        _ => {}
                    },
                    EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                        let points: Vec<TouchPoint> = self
                            .slots
                            .iter()
                            .flatten()
                            .filter(|info| info.initialized)
                            .map(|info| TouchPoint {
                                x: info.x as f64,
                                y: info.y as f64,
                                #[allow(dead_code)]
                                tracking_id: info.tracking_id,
                            })
                            .collect();
                        debug!("触控帧: {} 点", points.len());
                        return Ok(points);
                    }
                    _ => {}
                }
            }
        }
    }
}

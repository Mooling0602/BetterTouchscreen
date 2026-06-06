# BetterTouchscreen

将多点触控（触屏）输入转换为触摸板手势的 Linux 工具，专为 Wayland 桌面环境设计。

## 用法

当前仅实现了虚拟触摸板功能，执行 `cargo run` 可以启动虚拟触摸板，将触摸信号转换成触摸板手势事件。

> 暂时请使用 Ctrl + C 停止程序；若要避免使用 root 权限，可以将当前用户添加到 `input` 组。

配置文件位于 `~/.config/bettertouchscreen/config.toml`。

> 若使用sudo，则家目录位于 `/root` 而非 `/home/<user>`。

## 项目目标

在 Linux/Wayland 桌面上，触屏设备虽然支持多点触控，但第三方应用通常没有进行额外适配，**BetterTouchscreen** 的目标是通过一系列模块来改善触摸体验：

### 虚拟触摸板

- **监听** 触屏设备的原始多点触控事件
- **识别** 单指光标操作（移动、点击、双击拖拽）和双指滚动
- **转换为** 触摸板事件（光标移动、左/右键点击、垂直/水平滚动），使 Compositor 和 Wayland 应用能够响应

### 触摸手势

- **长按** 单指长按转换为鼠标右键
- ...（有待进一步讨论设计）

## 支持平台

- **仅 Wayland** — 不计划支持 X11
- 测试目标：wlroots-based compositors (Sway, Hyprland) 及 KDE/KWin Wayland、GNOME/Mutter

## 开发计划

- [x] 虚拟触摸板
- [ ] 触摸手势
- [ ] 触摸板与触摸屏模式热切换
- [ ] 图形化/桌面环境集成

## 技术栈

- **语言**: Rust
- **输入**: libinput（内核级输入事件）
- **输出**: uinput（Linux 内核虚拟输入设备）
- **平台**: Linux Wayland
- **构建系统**: Cargo

## 工作原理

```
触屏硬件 → evdev → libinput → 手势识别引擎 → uinput(虚拟触摸板) → Wayland Compositor → 应用
```

## 系统要求

- Linux 内核 5.0+
- Wayland 桌面环境
- 支持多点触控的触屏设备
- libinput 和 uinput 权限

## 许可证

GPL-3.0

# AGENTS.md — BetterTouchscreen 开发辅助

## 项目概述

将触屏多点触控事件转换为触摸板手势的 Rust 项目，**仅支持 Linux Wayland**。

## 开发环境

- **Rust 工具链**: stable（当前 1.96.0）
- **安装命令**: `cargo install --path .`
- **构建命令**: `cargo build`
- **运行命令**: `cargo run`
- **测试命令**: `cargo test`
- **代码检查**: `cargo clippy`
- **格式化**: `cargo fmt`

## 项目结构

执行`tree`查看源码结构。不建议在此处记录结果，因为容易过时；确需记录的话，请加上时间戳；不要删除这条提示！

## 依赖

见`Cargo.toml`。

## 平台支持

- **仅 Wayland** — 不考虑 X11 兼容性
- 不依赖特定 compositor 协议，使用内核级 evdev + uinput 方案

## 文档

- `doc/Usage.md` — 手势使用手册，定义所有手势类型、触发条件、输出映射、冲突解决规则

## 配置文件

支持 TOML 配置文件，默认存储在`~/.config/bettertouchscreen`下，也可以通过 `--config` 参数指定路径：

```toml
scroll_threshold = 5.0
scroll_sensitivity = 0.05
pointer_sensitivity = 0.5
hscroll_threshold = 2
debug_overlay = false
```

## 其他

应该仅将必要且不容易变化的信息记录在此处。对于重要但容易变化的信息，建议创建 CACHE.md 文件，并在开头记录更新时间和更新时间戳提示。

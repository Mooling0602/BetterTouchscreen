// 命令行参数解析 — 基于 clap derive

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "bettertouchscreen",
    about = "触屏多点触控转触摸板手势",
    version
)]
pub struct Cli {
    /// 指定触屏设备路径（如 /dev/input/event5）
    #[arg(short, long)]
    pub device: Option<String>,

    /// 列出所有支持多点触控的设备
    #[arg(short, long)]
    pub list: bool,

    /// 配置文件路径（TOML 格式），默认 ~/.config/bettertouchscreen/config.toml
    #[arg(short, long)]
    pub config: Option<String>,

    /// 生成示例配置文件到指定路径（默认 ~/.config/bettertouchscreen/config.toml）
    #[arg(long)]
    pub generate_config: Option<Option<String>>,

    /// 日志文件路径（默认 trace 级别，控制台为 info 级别）
    #[arg(long)]
    pub log_file: Option<String>,

    /// 启用调试叠加层，在屏幕上显示触控点位置
    #[arg(long)]
    pub debug_overlay: bool,
}

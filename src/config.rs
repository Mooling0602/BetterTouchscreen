// 配置管理 — 手势阈值参数 + TOML 文件加载

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 默认配置文件路径: ~/.config/bettertouchscreen/config.toml
pub fn default_config_path() -> Option<String> {
    dirs::config_dir().map(|p| {
        p.join("bettertouchscreen")
            .join("config.toml")
            .to_string_lossy()
            .to_string()
    })
}

/// 示例配置文件内容
pub const EXAMPLE_CONFIG: &str = r###"# BetterTouchscreen 配置文件
# 所有参数均可省略，省略时使用默认值

# 滚动检测阈值（设备像素），低于此值的移动不触发滚动
# scroll_threshold = 5.0

# 滑动检测阈值（设备像素）
# swipe_threshold = 50.0

# 滑动死区（设备像素）
# swipe_deadzone = 30.0

# 光标灵敏度，值越大光标移动越快
# pointer_sensitivity = 0.5

# 滚动灵敏度，值越大滚动越快
# scroll_sensitivity = 0.05
"###;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_scroll_threshold")]
    pub scroll_threshold: f64,
    #[serde(default = "default_swipe_threshold")]
    pub swipe_threshold: f64,
    #[serde(default = "default_swipe_deadzone")]
    pub swipe_deadzone: f64,
    /// 光标灵敏度，0.0~1.0，默认 0.5
    #[serde(default = "default_pointer_sensitivity")]
    pub pointer_sensitivity: f64,
    /// 滚动灵敏度，越小越慢，默认 0.05
    #[serde(default = "default_scroll_sensitivity")]
    pub scroll_sensitivity: f64,
}

fn default_scroll_threshold() -> f64 {
    5.0
}
fn default_swipe_threshold() -> f64 {
    50.0
}
fn default_swipe_deadzone() -> f64 {
    30.0
}

fn default_pointer_sensitivity() -> f64 {
    0.5
}

fn default_scroll_sensitivity() -> f64 {
    0.05
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scroll_threshold: default_scroll_threshold(),
            swipe_threshold: default_swipe_threshold(),
            swipe_deadzone: default_swipe_deadzone(),
            pointer_sensitivity: default_pointer_sensitivity(),
            scroll_sensitivity: default_scroll_sensitivity(),
        }
    }
}

impl Config {
    /// 从 TOML 文件加载配置
    /// - `Some(path)`: 使用指定路径
    /// - `None`: 尝试默认路径 ~/.config/bettertouchscreen/config.toml
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let config_path = match path {
            Some(p) => p.to_string(),
            None => match default_config_path() {
                Some(p) => p,
                None => return Ok(Self::default()),
            },
        };
        let path = Path::new(&config_path);
        if !path.exists() {
            // 自动创建默认配置文件
            if let Some(parent) = path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, EXAMPLE_CONFIG)?;
            log::info!("已创建默认配置文件: {}", path.display());
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("读取配置文件失败: {}: {}", path.display(), e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("解析配置文件失败: {}", e))?;
        log::info!("已加载配置文件: {}", path.display());
        Ok(config)
    }

    /// 生成示例配置文件
    pub fn generate_config(path: Option<&str>) -> anyhow::Result<()> {
        let config_path = match path {
            Some(p) => p.to_string(),
            None => default_config_path()
                .ok_or_else(|| anyhow::anyhow!("无法确定默认配置目录"))?,
        };
        let path = Path::new(&config_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            anyhow::bail!("配置文件已存在: {}", path.display());
        }
        std::fs::write(path, EXAMPLE_CONFIG)?;
        log::info!("已生成配置文件: {}", path.display());
        Ok(())
    }
}

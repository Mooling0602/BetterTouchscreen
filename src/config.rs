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

# 启用调试叠加层，在屏幕上显示触控点位置（需要 Wayland）
# debug_overlay = false

# 坐标轴变换（屏幕旋转/镜像补偿）
# 当触屏方向与显示方向不匹配时，通过以下三个参数校准
# 所有参数默认 false，可根据实际设备组合使用：
#   180° 旋转: invert_x = true, invert_y = true
#   90° 顺时针: swap_axes = true, invert_y = true
#   90° 逆时针: swap_axes = true, invert_x = true

# 交换 X / Y 轴（覆盖 90°/270° 旋转场景）
# swap_axes = false

# X 轴方向反转
# invert_x = false

# Y 轴方向反转
# invert_y = false

# 水平滚动最小阈值（设备像素），低于此值视为意外抖动，不触发 Shift 横向滚动
# hscroll_threshold = 2

# 触屏设备路径（可选，如 /dev/input/event6）
# 若注释或留空，程序将自动查找首个可用的多点触控设备
# touchscreen_device = "/dev/input/event26"
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
    /// 启用调试叠加层，在屏幕上显示触控点位置
    #[allow(dead_code)]
    #[serde(default)]
    pub debug_overlay: bool,
    /// 交换 X/Y 轴（覆盖 90°/270° 旋转场景）
    #[serde(default)]
    pub swap_axes: bool,
    /// X 轴方向反转
    #[serde(default)]
    pub invert_x: bool,
    /// Y 轴方向反转
    #[serde(default)]
    pub invert_y: bool,
    /// 水平滚动最小阈值，低于此值不触发 Shift 横向滚动，默认 2
    #[serde(default = "default_hscroll_threshold")]
    pub hscroll_threshold: i32,
    /// 触屏设备路径（如 /dev/input/event26），留空则自动识别
    #[serde(default)]
    pub touchscreen_device: Option<String>,
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

fn default_hscroll_threshold() -> i32 {
    2
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scroll_threshold: default_scroll_threshold(),
            swipe_threshold: default_swipe_threshold(),
            swipe_deadzone: default_swipe_deadzone(),
            pointer_sensitivity: default_pointer_sensitivity(),
            scroll_sensitivity: default_scroll_sensitivity(),
            debug_overlay: false,
            swap_axes: false,
            invert_x: false,
            invert_y: false,
            hscroll_threshold: default_hscroll_threshold(),
            touchscreen_device: None,
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
            None => default_config_path().ok_or_else(|| anyhow::anyhow!("无法确定默认配置目录"))?,
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

    /// 保存配置到文件
    ///
    /// 将当前配置序列化为 TOML 并写回配置文件。
    /// 使用的路径与 load 一致。
    pub fn save(&self, path: Option<&str>) -> anyhow::Result<()> {
        let config_path = match path {
            Some(p) => p.to_string(),
            None => match default_config_path() {
                Some(p) => p,
                None => anyhow::bail!("无法确定默认配置目录"),
            },
        };
        let path = Path::new(&config_path);
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str =
            toml::to_string_pretty(self).map_err(|e| anyhow::anyhow!("序列化配置失败: {}", e))?;
        std::fs::write(path, toml_str)?;
        log::info!("配置已保存到: {}", path.display());
        Ok(())
    }
}

// 触屏方向校准模块 — 引导用户滑动两次，自动计算坐标轴变换配置
//
// 工作流程:
//   1. 提示用户从下往上滑动，记录原始坐标 delta
//   2. 提示用户从左往右滑动，记录原始坐标 delta
//   3. 根据两次 delta 的主轴和方向，计算出 swap_axes / invert_x / invert_y
//   4. 将结果保存到配置文件

use crate::config::Config;
use crate::input::touch_input::TouchScreen;
use anyhow::Result;
use log::info;

/// 滑动距离必须超过此阈值才视为有效校准手势（设备像素）
const MIN_SWIPE_DISTANCE: f64 = 150.0;

/// 运行校准流程，返回更新后的配置
pub fn run_calibration(
    touchscreen: &mut TouchScreen,
    config_path: Option<&str>,
) -> Result<Config> {
    println!();
    println!("╔══════════════════════════════════════════╗");
    println!("║     触屏方向校准 — 屏幕旋转助手        ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("此工具将引导你完成两个滑动操作，");
    println!("自动计算正确的坐标轴变换配置。");
    println!();
    println!("注意：校准期间请勿使用其他手指触碰屏幕。");
    println!();

    // 阶段 1: 竖直方向校准
    let up_delta = calibrate_swipe(
        touchscreen,
        "【第 1 步】请用单指从屏幕底部向顶部竖直滑动",
        "等待手指按下...",
    )?;

    let (ux, uy) = up_delta;
    info!(
        "竖直滑动原始 delta: ({:.0}, {:.0})",
        ux, uy
    );
    println!(
        "  竖直滑动检测完成 — 原始位移: Δx={:.0}  Δy={:.0}",
        ux, uy
    );
    println!();

    // 阶段 2: 水平方向校准
    let right_delta = calibrate_swipe(
        touchscreen,
        "【第 2 步】请用单指从屏幕左侧向右侧水平滑动",
        "等待手指按下...",
    )?;

    let (rx, ry) = right_delta;
    info!(
        "水平滑动原始 delta: ({:.0}, {:.0})",
        rx, ry
    );
    println!(
        "  水平滑动检测完成 — 原始位移: Δx={:.0}  Δy={:.0}",
        rx, ry
    );
    println!();

    // 阶段 3: 计算变换参数
    let (swap_axes, invert_x, invert_y) = compute_transform(up_delta, right_delta);

    // 加载现有配置并更新
    let mut config = Config::load(config_path)?;
    config.swap_axes = swap_axes;
    config.invert_x = invert_x;
    config.invert_y = invert_y;

    // 保存配置
    config.save(config_path)?;

    println!("╔══════════════════════════════════════════╗");
    println!("║           校准完成！                    ║");
    println!("╠══════════════════════════════════════════╣");
    println!(
        "║  swap_axes = {:5}                      ║",
        swap_axes
    );
    println!(
        "║  invert_x  = {:5}                      ║",
        invert_x
    );
    println!(
        "║  invert_y  = {:5}                      ║",
        invert_y
    );
    println!("╠══════════════════════════════════════════╣");
    println!("║  配置已保存，可立即正常启动程序。      ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // 展示组合含义
    let rotation_desc = describe_transform(swap_axes, invert_x, invert_y);
    println!("当前方向校准: {}", rotation_desc);

    Ok(config)
}

/// 执行单次滑动校准，返回手指从按下到抬起的总位移 (dx, dy)
fn calibrate_swipe(
    touchscreen: &mut TouchScreen,
    instruction: &str,
    waiting_msg: &str,
) -> Result<(f64, f64)> {
    println!("{}", instruction);
    println!();

    loop {
        println!("  {} (最小滑动距离: {:.0} 像素)", waiting_msg, MIN_SWIPE_DISTANCE);

        let (start, end, _tracking_id) = detect_single_swipe(touchscreen)?;

        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < MIN_SWIPE_DISTANCE {
            println!(
                "  ⚠ 滑动距离不足 ({:.0} < {:.0} 像素)，请重新滑动",
                distance, MIN_SWIPE_DISTANCE
            );
            continue;
        }

        return Ok((dx, dy));
    }
}

/// 单次滑动检测结果: (起始位置, 结束位置, tracking_id)
type SwipeResult = ((f64, f64), (f64, f64), i32);

/// 检测一次完整的单指滑动: 按下 → 移动 → 抬起
///
/// 返回 (起始位置, 结束位置, tracking_id)。
/// 仅追踪第一个手指，忽略额外触控点。
fn detect_single_swipe(touchscreen: &mut TouchScreen) -> Result<SwipeResult> {
    // —— 阶段 A: 等待手指按下 ——
    let (start_pos, tracking_id) = loop {
        let touches = touchscreen.read_touch_frame()?;
        if let Some(first) = touches.first() {
            break ((first.x, first.y), first.tracking_id);
        }
    };

    // —— 阶段 B: 追踪手指移动直到抬起 ——
    let mut last_pos = start_pos;
    loop {
        let touches = touchscreen.read_touch_frame()?;

        // 找到属于同一 tracking_id 的触控点（可能已抬起）
        let tracked = touches.iter().find(|t| t.tracking_id == tracking_id);

        match tracked {
            Some(tp) => {
                // 手指仍在屏幕上，更新最后已知位置
                last_pos = (tp.x, tp.y);
            }
            None => {
                // 手指已抬起，结束
                break;
            }
        }
    }

    Ok((start_pos, last_pos, tracking_id))
}

/// 根据两次滑动的原始坐标 delta 计算坐标轴变换参数
///
/// # 参数
/// * `up_delta` — 向上滑动时原始设备的 (dx, dy)
/// * `right_delta` — 向右滑动时原始设备的 (dx, dy)
///
/// # 算法
///
/// 首先判断轴是否交换：
/// - 如果向上滑动以 X 轴为主（|ux| > |uy|）且向右滑动以 Y 轴为主（|ry| > |rx|），
///   则说明需要交换 X/Y 轴。
///
/// 然后判断每个轴是否需要反转：
/// - 非交换模式下：向右滑动时 raw_dx 应为正，否则 invert_x = true；
///   向上滑动时 raw_dy 应为负，否则 invert_y = true。
/// - 交换模式下：向右滑动时 raw_dy 经 swap 将成为输出 dx，应为正；
///   向上滑动时 raw_dx 经 swap 将成为输出 dy，应为负。
fn compute_transform(
    up_delta: (f64, f64),
    right_delta: (f64, f64),
) -> (bool, bool, bool) {
    let (ux, uy) = up_delta;
    let (rx, ry) = right_delta;

    // 判断是否需要交换轴
    // 条件：向上滑动的主轴在 X，且向右滑动的主轴在 Y
    let swap_axes = ux.abs() > uy.abs() && ry.abs() > rx.abs();

    let (invert_x, invert_y) = if swap_axes {
        // swap 后 raw_dy → 输出 dx: 向右滑动时应为正
        // swap 后 raw_dx → 输出 dy: 向上滑动时应为负
        (ry < 0.0, ux > 0.0)
    } else {
        // raw_dx → 输出 dx: 向右滑动时应为正
        // raw_dy → 输出 dy: 向上滑动时应为负
        (rx < 0.0, uy > 0.0)
    };

    (swap_axes, invert_x, invert_y)
}

/// 用中文描述当前旋转变换的含义
fn describe_transform(swap_axes: bool, invert_x: bool, invert_y: bool) -> &'static str {
    match (swap_axes, invert_x, invert_y) {
        (false, false, false) => "无旋转（原始方向）",
        (false, true, false) => "水平镜像（X 轴反转）",
        (false, false, true) => "垂直镜像（Y 轴反转）",
        (false, true, true) => "180° 旋转",
        (true, false, false) => "90° 逆时针旋转",
        (true, false, true) => "90° 顺时针旋转",
        (true, true, false) => "90° 逆时针 + 水平镜像",
        (true, true, true) => "90° 顺时针 + 垂直镜像",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_orientation() {
        // 正常屏幕: 上滑 → raw_dy 负，右滑 → raw_dx 正
        let up = (5.0, -500.0); // Y 为主轴，负方向
        let right = (500.0, 10.0); // X 为主轴，正方向
        let (swap, ix, iy) = compute_transform(up, right);
        assert!(!swap);
        assert!(!ix); // rx > 0 → 不需要反转
        assert!(!iy); // uy < 0 → 不需要反转
    }

    #[test]
    fn rotation_180() {
        // 180° 旋转: 上滑 → raw_dy 正（倒置），右滑 → raw_dx 负
        let up = (-10.0, 500.0);
        let right = (-500.0, 5.0);
        let (swap, ix, iy) = compute_transform(up, right);
        assert!(!swap);
        assert!(ix); // rx < 0
        assert!(iy); // uy > 0
    }

    #[test]
    fn rotation_90_cw() {
        // 90° 顺时针: 上滑 → 以 X 为主轴，正值；右滑 → 以 Y 为主轴，正值
        let up = (500.0, 10.0);
        let right = (-5.0, 500.0);
        let (swap, ix, iy) = compute_transform(up, right);
        assert!(swap);
        assert!(!ix); // ry > 0
        assert!(iy); // ux > 0
    }

    #[test]
    fn rotation_90_ccw() {
        // 90° 逆时针: 上滑 → 以 X 为主轴，负值；右滑 → 以 Y 为主轴，负值
        let up = (-500.0, 8.0);
        let right = (3.0, -500.0);
        let (swap, ix, iy) = compute_transform(up, right);
        assert!(swap);
        assert!(ix); // ry < 0
        assert!(!iy); // ux < 0
    }

    #[test]
    fn describe_all_combos() {
        assert_eq!(describe_transform(false, false, false), "无旋转（原始方向）");
        assert_eq!(describe_transform(false, true, false), "水平镜像（X 轴反转）");
        assert_eq!(describe_transform(false, false, true), "垂直镜像（Y 轴反转）");
        assert_eq!(describe_transform(false, true, true), "180° 旋转");
        assert_eq!(describe_transform(true, false, false), "90° 逆时针旋转");
        assert_eq!(describe_transform(true, false, true), "90° 顺时针旋转");
        assert_eq!(describe_transform(true, true, false), "90° 逆时针 + 水平镜像");
        assert_eq!(describe_transform(true, true, true), "90° 顺时针 + 垂直镜像");
    }
}

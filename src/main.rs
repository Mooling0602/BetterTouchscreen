use anyhow::Result;
use bettertouchscreen::cli::Cli;
use bettertouchscreen::config::Config;
use bettertouchscreen::gesture::engine::GestureEngine;
use bettertouchscreen::input::touch_input::{self, TouchScreen};
use bettertouchscreen::output::virtpad::VirtualTouchpad;
use clap::Parser;
use fern::colors::{Color, ColoredLevelConfig};
use log::{error, info};

fn init_logging(log_file: Option<&str>) -> Result<()> {
    // 默认日志文件：~/.cache/bettertouchscreen/bettertouchscreen.log
    let log_path = match log_file {
        Some(path) => path.to_string(),
        None => {
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("bettertouchscreen");
            std::fs::create_dir_all(&cache_dir)?;
            cache_dir
                .join("bettertouchscreen.log")
                .to_string_lossy()
                .to_string()
        }
    };
    let console_colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red)
        .debug(Color::Blue)
        .trace(Color::White);

    // 控制台输出：info 级别
    let console = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                console_colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout());

    let mut dispatch = fern::Dispatch::new().chain(console);

    // 文件输出：trace 级别
    let file = fern::log_file(log_path)?;
    let file_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{:5}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(file);
    dispatch = dispatch.chain(file_dispatch);

    dispatch.apply()?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(cli.log_file.as_deref())?;

    if cli.list {
        let devices = touch_input::list_touchscreens();
        if devices.is_empty() {
            println!("未找到支持多点触控的设备");
        } else {
            println!("支持多点触控的设备:");
            for (path, name, is_direct) in &devices {
                let tag = if *is_direct { "触屏" } else { "触摸板" };
                println!("  {} — {} [{}]", path, name, tag);
            }
        }
        return Ok(());
    }

    // 生成配置文件
    if let Some(path_opt) = cli.generate_config {
        let path = path_opt.as_deref();
        Config::generate_config(path)?;
        return Ok(());
    }

    let config = Config::load(cli.config.as_deref())?;
    info!("BetterTouchscreen — 触屏多点触控 → 触摸板手势");
    info!("配置: {:?}", config);

    let mut touchscreen = if let Some(path) = &cli.device {
        TouchScreen::open(path)?
    } else {
        TouchScreen::find_touchscreen()?
    };

    info!("触屏设备: {}", touchscreen.name().unwrap_or("未知"));

    let mut engine = GestureEngine::new(config);
    let mut virtpad = VirtualTouchpad::new()?;

    info!("开始监听触控事件...");

    loop {
        match touchscreen.read_touch_frame() {
            Ok(touches) => {
                let events = engine.process(&touches);
                for event in &events {
                    if let Err(e) = virtpad.emit_gesture(event) {
                        error!("输出手势事件失败: {}", e);
                    }
                }
            }
            Err(e) => {
                // 检查是否为设备断开错误 (ENODEV)
                if let Some(io_err) = e.downcast_ref::<std::io::Error>()
                    && io_err.raw_os_error() == Some(libc::ENODEV)
                {
                    error!("触屏设备已断开 (ENODEV)，程序退出");
                    std::process::exit(1);
                }
                error!("读取触控事件失败: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

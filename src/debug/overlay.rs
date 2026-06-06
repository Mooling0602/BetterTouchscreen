use crate::types::TouchPoint;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::os::fd::{AsFd, OwnedFd};
use std::sync::{Arc, Mutex};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::WlCallback,
        wl_compositor::WlCompositor,
        wl_registry::WlRegistry,
        wl_region::WlRegion,
        wl_shm::{self, Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

const CIRCLE_RADIUS: i32 = 20;
const CIRCLE_BORDER: i32 = 3;
const CIRCLE_ALPHA: u8 = 200;

const COLOR_PALETTE: [(u8, u8, u8); 8] = [
    (255, 60, 60),
    (60, 180, 60),
    (60, 120, 255),
    (255, 200, 40),
    (200, 60, 255),
    (60, 220, 220),
    (255, 140, 40),
    (255, 80, 180),
];

pub struct TouchOverlay {
    touches: Arc<Mutex<Vec<TouchPoint>>>,
}

impl TouchOverlay {
    pub fn new(touch_max_x: f64, touch_max_y: f64) -> Result<Self> {
        let touches = Arc::new(Mutex::new(Vec::new()));
        let touches_clone = touches.clone();

        std::thread::Builder::new()
            .name("bts-overlay".into())
            .spawn(move || {
                if let Err(e) = run_overlay(touches_clone, touch_max_x, touch_max_y) {
                    error!("叠加层线程退出: {}", e);
                }
            })
            .context("无法创建叠加层线程")?;

        info!("调试叠加层已启动 (设备最大坐标: {}x{})", touch_max_x, touch_max_y);
        Ok(Self { touches })
    }

    pub fn update(&self, points: Vec<TouchPoint>) {
        if let Ok(mut guard) = self.touches.lock() {
            *guard = points;
        }
    }
}

// ── State ──────────────────────────────────────────────────────────

struct OverlayState {
    running: bool,
    needs_redraw: bool,
    width: u32,
    height: u32,
    scale_x: f64,
    scale_y: f64,
    touch_max_x: f64,
    touch_max_y: f64,
    shm: Option<WlShm>,
    surface: Option<WlSurface>,
    pool: Option<WlShmPool>,
    pool_fd: Option<OwnedFd>,
    mmap: Option<memmap2::MmapMut>,
    touches: Arc<Mutex<Vec<TouchPoint>>>,
}

// ── Dispatch impls ─────────────────────────────────────────────────

impl Dispatch<WlShm, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlShm, _e: wl_shm::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlCallback, ()> for OverlayState {
    fn event(
        s: &mut Self, _p: &WlCallback,
        _e: <WlCallback as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {
        s.needs_redraw = true;
    }
}

impl Dispatch<WlSurface, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlSurface,
        _e: <WlSurface as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlCompositor, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlCompositor,
        _e: <WlCompositor as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlShmPool, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlShmPool,
        _e: <WlShmPool as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &ZwlrLayerShellV1,
        _e: <ZwlrLayerShellV1 as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlRegistry, GlobalListContents> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlRegistry, _e: <WlRegistry as wayland_client::Proxy>::Event,
        _u: &GlobalListContents, _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlBuffer, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlBuffer, _e: <WlBuffer as wayland_client::Proxy>::Event,
        _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlRegion, ()> for OverlayState {
    fn event(
        _s: &mut Self, _p: &WlRegion, _e: <WlRegion as wayland_client::Proxy>::Event,
        _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for OverlayState {
    fn event(
        s: &mut Self, p: &ZwlrLayerSurfaceV1,
        e: <ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event, _u: &(), _c: &Connection, _h: &QueueHandle<Self>,
    ) {
        match e {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                info!("叠加层配置: {}x{}", width, height);
                s.width = width;
                s.height = height;
                if s.touch_max_x > 0.0 && s.touch_max_y > 0.0 {
                    s.scale_x = width as f64 / s.touch_max_x;
                    s.scale_y = height as f64 / s.touch_max_y;
                    info!("坐标缩放: device({}x{}) → screen({}x{}), scale=({:.4}x{:.4})",
                        s.touch_max_x as u32, s.touch_max_y as u32,
                        width, height, s.scale_x, s.scale_y);
                }
                s.needs_redraw = true;
                p.ack_configure(serial);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                s.running = false;
            }
            _ => {}
        }
    }
}

// ── SHM pool ───────────────────────────────────────────────────────

fn create_shm_pool(
    shm: &WlShm,
    width: u32,
    height: u32,
    qh: &QueueHandle<OverlayState>,
) -> Result<(WlShmPool, OwnedFd, memmap2::MmapMut)> {
    let stride = width as i32 * 4;
    let size = (stride * height as i32) as u64;

    let file = tempfile::tempfile().context("创建 SHM 临时文件失败")?;
    file.set_len(size).context("设置 SHM 大小失败")?;

    let fd = OwnedFd::from(file);
    let mmap = unsafe {
        memmap2::MmapOptions::new()
            .len(size as usize)
            .map_mut(&fd)
            .context("mmap SHM 失败")?
    };

    let pool = shm.create_pool(fd.as_fd(), size as i32, qh, ());

    Ok((pool, fd, mmap))
}

// ── Render ─────────────────────────────────────────────────────────

fn render_frame(state: &mut OverlayState, qh: &QueueHandle<OverlayState>) -> Result<()> {
    let w = state.width;
    let h = state.height;
    if w == 0 || h == 0 {
        return Ok(());
    }

    let shm = state.shm.as_ref().context("wl_shm 未绑定")?;
    let surface = state.surface.as_ref().context("wl_surface 未创建")?;
    let stride = w as i32 * 4;
    let size = (stride * h as i32) as usize;

    // Recreate pool if size changed
    let need_new = state.mmap.as_ref().is_none_or(|m| m.len() < size);
    if need_new {
        let (pool, fd, mmap) = create_shm_pool(shm, w, h, qh)?;
        state.pool = Some(pool);
        state.pool_fd = Some(fd);
        state.mmap = Some(mmap);
    }

    let mmap = state.mmap.as_mut().unwrap();
    let buf_u32 = unsafe {
        std::slice::from_raw_parts_mut(mmap.as_mut_ptr() as *mut u32, size / 4)
    };
    buf_u32.fill(0);

    let points = state.touches.lock().unwrap().clone();
    if !points.is_empty() {
        let coords: Vec<String> = points.iter().map(|p| format!("({:.0}, {:.0})", p.x, p.y)).collect();
        debug!("渲染 {} 个触点: {}", points.len(), coords.join(" "));
    }
    for (i, point) in points.iter().enumerate() {
        let (r, g, b) = COLOR_PALETTE[i % COLOR_PALETTE.len()];
        let sx = (point.x * state.scale_x) as i32;
        let sy = (point.y * state.scale_y) as i32;
        draw_circle(buf_u32, w as usize, h as usize, sx, sy, CIRCLE_RADIUS, (r, g, b, CIRCLE_ALPHA));
    }

    let pool = state.pool.as_ref().unwrap();
    let buffer = pool.create_buffer(0, w as i32, h as i32, stride, Format::Argb8888, qh, ());

    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, w as i32, h as i32);
    surface.frame(qh, ());
    surface.commit();

    Ok(())
}

// ── Main loop ──────────────────────────────────────────────────────

fn run_overlay(touches: Arc<Mutex<Vec<TouchPoint>>>, touch_max_x: f64, touch_max_y: f64) -> Result<()> {
    let conn = Connection::connect_to_env().context("无法连接 Wayland display")?;
    let (globals, mut event_queue) = registry_queue_init(&conn).context("Wayland registry 初始化失败")?;
    let qh = event_queue.handle();

    // Bind globals
    let compositor: WlCompositor = globals.bind(&qh, 1..=6, ()).context("wl_compositor 不可用")?;
    let shm: WlShm = globals.bind(&qh, 1..=2, ()).context("wl_shm 不可用")?;
    let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=5, ()).context("zwlr_layer_shell_v1 不可用（compositor 不支持 wlr-layer-shell）")?;
    info!("Wayland 全局已绑定: compositor, shm, layer_shell");

    // Create surface
    let surface = compositor.create_surface(&qh, ());

    // Create layer surface
    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None, // output
        zwlr_layer_shell_v1::Layer::Overlay,
        "bts-debug".to_string(),
        &qh,
        (),
    );

    layer_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Bottom
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layer_surface.set_size(0, 0);
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

    // Empty input region = click-through
    let region = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&region));
    region.destroy();
    surface.commit();

    let mut state = OverlayState {
        running: true,
        needs_redraw: false,
        width: 0,
        height: 0,
        scale_x: 1.0,
        scale_y: 1.0,
        touch_max_x,
        touch_max_y,
        shm: Some(shm),
        surface: Some(surface),
        pool: None,
        pool_fd: None,
        mmap: None,
        touches,
    };

    info!("Wayland 叠加层已初始化 (wlr-layer-shell)");

    while state.running {
        if let Err(e) = event_queue.blocking_dispatch(&mut state) {
            error!("Wayland dispatch 错误: {}", e);
            break;
        }

        if state.needs_redraw {
            state.needs_redraw = false;
            if let Err(e) = render_frame(&mut state, &qh) {
                warn!("叠加层渲染失败: {}", e);
            }
        }
    }

    info!("叠加层线程正常退出");
    Ok(())
}

// ── Circle drawing ─────────────────────────────────────────────────

fn draw_circle(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    r: i32,
    color: (u8, u8, u8, u8), // (r, g, b, a)
) {
    let (cr, cg, cb, ca) = color;
    let fill = (ca as u32) << 24 | (cr as u32) << 16 | (cg as u32) << 8 | cb as u32;
    let border = 0xFF_000000u32;
    let outer = r + CIRCLE_BORDER;
    for dy in -outer..=outer {
        for dx in -outer..=outer {
            let px = cx + dx;
            let py = cy + dy;
            if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 { continue; }
            let d2 = dx * dx + dy * dy;
            let idx = py as usize * w + px as usize;
            if d2 <= r * r { buf[idx] = fill; }
            else if d2 <= outer * outer { buf[idx] = border; }
        }
    }
}

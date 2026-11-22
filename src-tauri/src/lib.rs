mod collect;

#[cfg(windows)]
mod windowing;

use collect::Collector;
use fanal_core::{classify, AgentId, AgentSnapshot, Evidence, Lamp, Settings, Severity};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::image::Image;
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{Manager, State, WebviewWindow};

struct AppState {
    collector: Mutex<Collector>,
    settings: Mutex<Settings>,
}

#[derive(Serialize)]
struct Board {
    agents: Vec<AgentSnapshot>,
    severity: Severity,
    settings: Settings,
}

fn board(state: &AppState) -> Board {
    let settings = state.settings.lock().expect("settings").clone();
    let observations = state.collector.lock().expect("collector").observe();
    let agents = classify(
        &observations,
        &settings,
        Evidence {
            now_ms: collect::now_ms(),
            stale_after_ms: Evidence::DEFAULT_STALE_MS,
        },
    );
    let severity = Severity::from_snapshots(&agents);
    Board {
        agents,
        severity,
        settings,
    }
}

#[tauri::command]
fn snapshot(state: State<AppState>) -> Board {
    board(&state)
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    collect::save_settings(&settings).map_err(|err| err.to_string())?;
    *state.settings.lock().expect("settings") = settings;
    Ok(())
}

#[tauri::command]
fn open_agent(id: AgentId) -> Result<(), String> {
    #[cfg(windows)]
    {
        let pids = collect::running_pids(id);
        if windowing::focus(&pids) {
            return Ok(());
        }
        // Not running, or running without a window: start it, then focus.
        if let Some(path) = collect::executable_for(id) {
            if let Some(new_pid) = windowing::launch(&path) {
                std::thread::sleep(std::time::Duration::from_millis(900));
                windowing::focus(&[new_pid]);
                return Ok(());
            }
        }
        return Err("no window to open".to_string());
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Err("opening a window is only implemented on Windows".to_string())
    }
}

/// A 32px lamp, the same colour the panel would show for this severity.
fn lamp_icon(lamp: Lamp) -> Image<'static> {
    const SIZE: usize = 32;
    let (red, green, blue) = match lamp {
        Lamp::Green => (48, 209, 88),
        Lamp::Yellow => (255, 159, 10),
        Lamp::Red => (255, 69, 58),
        Lamp::Grey => (139, 142, 152),
    };
    let mut pixels = vec![0u8; SIZE * SIZE * 4];
    let center = (SIZE as f32 - 1.0) / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = ((11.5 - distance) * 255.0).clamp(0.0, 255.0) as u8;
            let index = (y * SIZE + x) * 4;
            pixels[index] = red;
            pixels[index + 1] = green;
            pixels[index + 2] = blue;
            pixels[index + 3] = alpha;
        }
    }
    Image::new_owned(pixels, SIZE as u32, SIZE as u32)
}

fn tooltip_for(board: &Board) -> String {
    match board.severity.lamp {
        Lamp::Red => format!("Fanal \u{2014} {} waiting", board.severity.attention),
        Lamp::Green => format!("Fanal \u{2014} {} running", board.severity.running),
        Lamp::Yellow => format!("Fanal \u{2014} {} idle", board.severity.idle),
        Lamp::Grey => "Fanal".to_string(),
    }
}

fn watch(app: tauri::AppHandle, tray: TrayIcon) {
    std::thread::spawn(move || {
        let mut shown = None;
        loop {
            let current = board(app.state::<AppState>().inner());
            if shown != Some(current.severity.lamp) {
                shown = Some(current.severity.lamp);
                let _ = tray.set_icon(Some(lamp_icon(current.severity.lamp)));
                let _ = tray.set_tooltip(Some(tooltip_for(&current)));
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

/// The panel anchors to the top centre of the monitor it is on. The width is
/// passed in rather than read back: the size just set has not landed yet, and
/// centring on the old one shoves a shrinking window off the screen.
fn place(window: &WebviewWindow, width: f64) {
    if let (Ok(Some(monitor)), Ok(scale)) = (window.current_monitor(), window.scale_factor()) {
        let area = monitor.work_area();
        let wide = (width * scale).round() as i32;
        let x = area.position.x + (area.size.width as i32 - wide) / 2;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, area.position.y));
    }
}

#[tauri::command]
fn fit_panel(window: WebviewWindow, width: f64, height: f64) {
    // The window is exactly the painted panel. A taller transparent window sits
    // on top of whatever is underneath, so a collapsed lip used to cover the
    // browser tabs below it.
    place(&window, width);
    let _ = window.set_size(tauri::LogicalSize::new(width, height.max(11.0)));
}

pub fn run() {
    let state = AppState {
        collector: Mutex::new(Collector::new()),
        settings: Mutex::new({
            let mut settings = collect::load_settings();
            settings.reconcile();
            settings
        }),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_settings,
            open_agent,
            fit_panel
        ])
        .setup(|app| {
            let tray = TrayIconBuilder::with_id("fanal")
                .icon(lamp_icon(Lamp::Grey))
                .tooltip("Fanal")
                .build(app)?;
            watch(app.handle().clone(), tray);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("fanal failed to start")
        .run(|app, event| match event {
            tauri::RunEvent::Ready => {
                if let Some(window) = app.get_webview_window("main") {
                    place(&window, 132.0);
                    let _ = window.show();
                }
            }
            tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::Focused(focused),
                ..
            } => {
                if let Some(window) = app.get_webview_window("main") {
                    let script = if focused {
                        "window.__fanalFocus && window.__fanalFocus(true)"
                    } else {
                        "window.__fanalFocus && window.__fanalFocus(false)"
                    };
                    let _ = window.eval(script);
                }
            }
            _ => {}
        });
}

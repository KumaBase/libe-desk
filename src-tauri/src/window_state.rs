use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{App, AppHandle, Manager, Runtime, WebviewWindowBuilder, Window, WindowEvent};

const STATE_FILE: &str = "window-state.json";
const MIN_WIDTH: u32 = 760;
const MIN_HEIGHT: u32 = 520;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct WindowState {
    #[serde(default)]
    normal_size: Option<WindowSize>,
    #[serde(default)]
    maximized: bool,
    #[serde(default)]
    normal_position: Option<WindowPosition>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct WindowSize {
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
struct WindowPosition {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
struct WorkArea {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl WorkArea {
    fn from_monitor(monitor: &tauri::Monitor) -> Self {
        let area = monitor.work_area();
        Self {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        }
    }

    fn contains(self, position: WindowPosition) -> bool {
        let x = i64::from(position.x);
        let y = i64::from(position.y);
        x >= i64::from(self.x)
            && y >= i64::from(self.y)
            && x < i64::from(self.x) + i64::from(self.width)
            && y < i64::from(self.y) + i64::from(self.height)
    }
}

// 座標と外枠サイズは物理ピクセルで揃える。負の座標のモニターも扱う。
fn fit_position(
    position: WindowPosition,
    width: u32,
    height: u32,
    area: WorkArea,
) -> WindowPosition {
    let max_x = i64::from(area.x) + i64::from(area.width.saturating_sub(width));
    let max_y = i64::from(area.y) + i64::from(area.height.saturating_sub(height));
    WindowPosition {
        x: i64::from(position.x).clamp(i64::from(area.x), max_x) as i32,
        y: i64::from(position.y).clamp(i64::from(area.y), max_y) as i32,
    }
}

struct WindowStateStore {
    path: PathBuf,
    state: Arc<Mutex<WindowState>>,
}

pub fn create_main<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let path = app.path().app_config_dir()?.join(STATE_FILE);
    let mut saved = load(&path).unwrap_or_default();
    let mut config = app.config().app.windows[0].clone();
    let monitors = app.available_monitors()?;
    let monitor = saved
        .normal_position
        .and_then(|position| {
            monitors
                .iter()
                .find(|monitor| WorkArea::from_monitor(monitor).contains(position))
                .cloned()
        })
        .or(app.primary_monitor()?);
    let available = monitor.as_ref().map(|monitor| {
        let size = monitor
            .work_area()
            .size
            .to_logical::<f64>(monitor.scale_factor());
        WindowSize {
            width: size.width,
            height: size.height,
        }
    });
    let size = saved
        .normal_size
        .filter(|size| is_safe_size(*size, available))
        .unwrap_or(WindowSize {
            width: config.width,
            height: config.height,
        });
    config.width = available.map_or(size.width, |area| size.width.min(area.width));
    config.height = available.map_or(size.height, |area| size.height.min(area.height));
    let visible = config.visible;
    config.visible = false;
    config.maximized = false;
    saved.normal_size = Some(WindowSize {
        width: config.width,
        height: config.height,
    });
    WebviewWindowBuilder::from_config(app, &config)?.build()?;
    let window = app
        .get_window("main")
        .expect("main window was just created");
    if let (Some(position), Some(monitor)) = (saved.normal_position, monitor.as_ref()) {
        let outer = window.outer_size()?;
        let position = fit_position(
            position,
            outer.width,
            outer.height,
            WorkArea::from_monitor(monitor),
        );
        window.set_position(tauri::PhysicalPosition::new(position.x, position.y))?;
        saved.normal_position = Some(position);
    }
    if saved.maximized {
        window.maximize()?;
    }
    if visible {
        window.show()?;
    }
    let state = Arc::new(Mutex::new(saved));
    let state_for_events = Arc::clone(&state);
    let window_for_events = window.clone();
    let path_for_events = path.clone();
    window.app_handle().manage(WindowStateStore {
        path,
        state: Arc::clone(&state),
    });
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(size) => {
            if is_normal_window(&window_for_events) {
                let mut saved = state_for_events.lock().unwrap();
                saved.normal_size = logical_size(*size, &window_for_events);
            }
        }
        WindowEvent::Moved(position) => {
            if is_normal_window(&window_for_events) {
                state_for_events.lock().unwrap().normal_position = Some(WindowPosition {
                    x: position.x,
                    y: position.y,
                });
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let mut saved = state_for_events.lock().unwrap();
            saved.maximized = window_for_events.is_maximized().unwrap_or(false);
            if is_normal_window(&window_for_events) {
                saved.normal_size = window_size(&window_for_events).or(saved.normal_size);
                saved.normal_position =
                    window_position(&window_for_events).or(saved.normal_position);
            }
            if let Err(err) = save(&path_for_events, &saved) {
                eprintln!("window state save failed: {err}");
            }
        }
        _ => {}
    });
    Ok(())
}

pub fn save_on_exit<R: Runtime>(app: &AppHandle<R>) {
    let Some(store) = app.try_state::<WindowStateStore>() else {
        return;
    };
    let mut saved = store.state.lock().unwrap();
    if let Some(window) = app.get_window("main") {
        saved.maximized = window.is_maximized().unwrap_or(false);
        if is_normal_window(&window) {
            saved.normal_size = window_size(&window).or(saved.normal_size);
            saved.normal_position = window_position(&window).or(saved.normal_position);
        }
    }
    if let Err(err) = save(&store.path, &saved) {
        eprintln!("window state save failed: {err}");
    }
}

fn load(path: &PathBuf) -> Option<WindowState> {
    fs::read(path)
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
}

fn save(path: &PathBuf, state: &WindowState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec(state).expect("window state serializes"),
    )?;
    fs::rename(temporary, path)
}

fn is_normal_window<R: Runtime>(window: &Window<R>) -> bool {
    !window.is_maximized().unwrap_or(true)
        && !window.is_minimized().unwrap_or(true)
        && !window.is_fullscreen().unwrap_or(true)
}

fn window_position<R: Runtime>(window: &Window<R>) -> Option<WindowPosition> {
    let position = window.outer_position().ok()?;
    Some(WindowPosition {
        x: position.x,
        y: position.y,
    })
}

fn window_size<R: Runtime>(window: &Window<R>) -> Option<WindowSize> {
    let size = window.inner_size().ok()?;
    logical_size(size, window)
}

fn logical_size<R: Runtime>(
    size: tauri::PhysicalSize<u32>,
    window: &Window<R>,
) -> Option<WindowSize> {
    let size = size.to_logical::<f64>(window.scale_factor().ok()?);
    Some(WindowSize {
        width: size.width,
        height: size.height,
    })
}

fn is_safe_size(size: WindowSize, available: Option<WindowSize>) -> bool {
    size.width >= f64::from(MIN_WIDTH)
        && size.height >= f64::from(MIN_HEIGHT)
        && size.width.is_finite()
        && size.height.is_finite()
        && available.is_none_or(|available| {
            size.width <= available.width && size.height <= available.height
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_visible_positions_and_supports_negative_monitor_coordinates() {
        let area = WorkArea {
            x: -1920,
            y: 24,
            width: 1920,
            height: 1056,
        };
        let position = WindowPosition { x: -1800, y: 100 };
        assert!(area.contains(position));
        assert_eq!(fit_position(position, 1000, 700, area), position);
    }

    #[test]
    fn moves_offscreen_windows_into_available_work_area() {
        let area = WorkArea {
            x: 0,
            y: 24,
            width: 1440,
            height: 876,
        };
        assert_eq!(
            fit_position(WindowPosition { x: 2200, y: 1100 }, 1000, 700, area),
            WindowPosition { x: 440, y: 200 }
        );
        assert_eq!(
            fit_position(WindowPosition { x: -1900, y: -100 }, 2000, 1000, area),
            WindowPosition { x: 0, y: 24 }
        );
    }

    #[test]
    fn old_state_without_position_remains_readable() {
        let state: WindowState = serde_json::from_str(
            r#"{"normal_size":{"width":1400,"height":900},"maximized":false}"#,
        )
        .unwrap();
        assert_eq!(state.normal_position, None);
        assert_eq!(state.normal_size.unwrap().width, 1400.0);
    }

    #[test]
    fn position_survives_serialization() {
        let state = WindowState {
            normal_position: Some(WindowPosition { x: -1200, y: 80 }),
            ..Default::default()
        };
        let restored: WindowState =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        assert_eq!(restored.normal_position, state.normal_position);
    }

    #[test]
    fn accepts_a_size_that_fits_the_current_work_area() {
        assert!(is_safe_size(
            WindowSize {
                width: 1400.0,
                height: 900.0,
            },
            Some(WindowSize {
                width: 1728.0,
                height: 1117.0,
            }),
        ));
    }

    #[test]
    fn rejects_too_small_or_off_screen_sizes() {
        assert!(!is_safe_size(
            WindowSize {
                width: 1.0,
                height: 900.0,
            },
            Some(WindowSize {
                width: 1728.0,
                height: 1117.0,
            }),
        ));
        assert!(!is_safe_size(
            WindowSize {
                width: 1800.0,
                height: 900.0,
            },
            Some(WindowSize {
                width: 1728.0,
                height: 1117.0,
            }),
        ));
    }
}

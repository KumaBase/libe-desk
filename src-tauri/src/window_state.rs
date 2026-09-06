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
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct WindowSize {
    width: f64,
    height: f64,
}

struct WindowStateStore {
    path: PathBuf,
    state: Arc<Mutex<WindowState>>,
}

pub fn create_main<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let path = app.path().app_config_dir()?.join(STATE_FILE);
    let mut saved = load(&path).unwrap_or_default();
    let mut config = app.config().app.windows[0].clone();
    let available = app.primary_monitor()?.map(|monitor| {
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
    config.maximized = saved.maximized;
    saved.normal_size = Some(WindowSize {
        width: config.width,
        height: config.height,
    });
    WebviewWindowBuilder::from_config(app, &config)?.build()?;
    let window = app
        .get_window("main")
        .expect("main window was just created");
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
            if !window_for_events.is_maximized().unwrap_or(false) {
                let mut saved = state_for_events.lock().unwrap();
                saved.normal_size = logical_size(*size, &window_for_events);
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let mut saved = state_for_events.lock().unwrap();
            saved.maximized = window_for_events.is_maximized().unwrap_or(false);
            if !saved.maximized {
                saved.normal_size = window_size(&window_for_events).or(saved.normal_size);
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
        if !saved.maximized {
            saved.normal_size = window_size(&window).or(saved.normal_size);
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

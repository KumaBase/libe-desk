mod tab_manager;
mod window_state;

use tab_manager::TabManager;
use tauri::{Manager, RunEvent, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(TabManager::default())
        .setup(|app| {
            window_state::create_main(app)?;
            // 初期メニュー（タブ一覧は空）。タブ開閉のたびに rebuild する。
            tab_manager::rebuild_app_menu(app.handle(), &[])?;

            app.on_menu_event(|app, event| {
                let id = event.id().as_ref();
                if id.starts_with("tab-") {
                    if let Err(err) = tab_manager::handle_tab_menu(app, id) {
                        eprintln!("tab menu failed: {err}");
                    }
                    return;
                }
                let action = match id {
                    "nav-back" => "back",
                    "nav-forward" => "forward",
                    "nav-reload" => "reload",
                    _ => return,
                };
                if let Err(err) = tab_manager::navigate_active(app, action) {
                    eprintln!("menu navigation failed: {err}");
                }
            });

            let handle = app.handle().clone();
            if let Some(window) = app.get_window("main") {
                window.on_window_event(move |event| {
                    if let WindowEvent::Resized(_) = event {
                        let state = handle.state::<TabManager>();
                        let mut guard = state.lock().unwrap();
                        if let Err(err) =
                            tab_manager::apply_chrome_layout_inner(&handle, &mut guard)
                        {
                            eprintln!("layout on resize failed: {err}");
                        }
                    }
                });
            }

            {
                let state = app.state::<TabManager>();
                let mut guard = state.lock().unwrap();
                if let Err(err) =
                    tab_manager::apply_chrome_layout_inner_with(app.handle(), &mut guard, false)
                {
                    eprintln!("initial layout failed: {err}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tab_manager::list_services,
            tab_manager::open_service,
            tab_manager::list_tabs,
            tab_manager::switch_tab,
            tab_manager::move_tab,
            tab_manager::close_tab,
            tab_manager::go_back,
            tab_manager::go_forward,
            tab_manager::reload_tab,
            tab_manager::get_current_page,
            tab_manager::apply_chrome_layout,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Tauri application")
        .run(|app, event| match event {
            RunEvent::ExitRequested { .. } | RunEvent::Exit => window_state::save_on_exit(app),
            _ => {}
        });
}

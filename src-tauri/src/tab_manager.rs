//! タブ・Webview操作の共通コントローラ層。
//!
//! UI(SolidJS)はここで定義された tauri コマンドのみを呼び出し、
//! Webviewの生成・表示切替・ナビゲーション制御の詳細は知らない。
//! 将来 MCP サーバーを追加する場合も、同じ関数群を呼び出すアダプタを
//! 追加するだけで済むようにするための分離。

use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{
    webview::WebviewBuilder, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime,
    Url, WebviewUrl,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

/// サービス一覧の単一ソース。(id, 表示名, URL, カテゴリ, default_pinned)
///
/// default_pinned=true は初期状態で「よく使う」に出す候補。
/// 実際のピン留めはフロント側で管理し、リベシティのみ固定表示。
const SERVICES: &[(&str, &str, &str, &str, bool)] = &[
    // --- 本体 ---
    (
        "libecity",
        "リベシティ",
        "https://libecity.com/",
        "本体",
        true,
    ),
    (
        "chat",
        "参加チャット",
        "https://libecity.com/room_list",
        "本体",
        false,
    ),
    (
        "tweet",
        "つぶやき",
        "https://libecity.com/tweet/all",
        "本体",
        false,
    ),
    (
        "events",
        "イベント・オフ会カレンダー",
        "https://libecity.com/mypage/event_calendar",
        "本体",
        false,
    ),
    (
        "homework",
        "宿題リスト",
        "https://libecity.com/mypage/fiveforces",
        "本体",
        false,
    ),
    (
        "points",
        "リベポイント",
        "https://libecity.com/mypage/points",
        "本体",
        false,
    ),
    (
        "president-mag",
        "学長マガジン",
        "https://libecity.com/room_list?room_id=President-Tweet",
        "本体",
        false,
    ),
    (
        "president-invest",
        "学長高配当株マガジン",
        "https://libecity.com/room_list?room_id=President-investment-magazine",
        "本体",
        false,
    ),
    (
        "guide",
        "リベシティ案内所",
        "https://libecity.com/room_list?room_id=FirstStep",
        "本体",
        false,
    ),
    (
        "announce",
        "運営からのお知らせ",
        "https://libecity.com/room_list?room_id=Liberal-City",
        "本体",
        false,
    ),
    (
        "bookmarks",
        "ブックマーク",
        "https://libecity.com/bookmark",
        "本体",
        false,
    ),
    (
        "lifeplan",
        "支出管理・ライフプラン",
        "https://lifeplan.libecity.com/",
        "本体",
        false,
    ),
    // --- 学ぶ ---
    (
        "library",
        "リベシティノウハウ図書館",
        "https://library.libecity.com/",
        "学ぶ",
        false,
    ),
    (
        "school-list",
        "リベシティオンラインスクール",
        "https://site.libecity.com/school-list",
        "学ぶ",
        false,
    ),
    (
        "seminars",
        "リベシティセミナー",
        "https://site.libecity.com/seminar-category/seminar-list",
        "学ぶ",
        false,
    ),
    (
        "camps",
        "リベシティ合宿",
        "https://site.libecity.com/camp-category/camp-list",
        "学ぶ",
        false,
    ),
    // --- 交流する ---
    (
        "office",
        "リベシティオフィス",
        "https://office.libecity.com/",
        "交流する",
        false,
    ),
    (
        "ovice",
        "リベシティオンラインスペース（ovice）",
        "https://libecity.com/room_list?room_id=VirtualOffice",
        "交流する",
        false,
    ),
    (
        "map",
        "リベシティマップ",
        "https://map.libecity.com/login",
        "交流する",
        false,
    ),
    // --- 仕事・副業・売買 ---
    (
        "works",
        "リベシティワークス",
        "https://works.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "skill",
        "リベシティスキルマーケットOnline",
        "https://skill.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "meets",
        "リベシティスキルマーケットMeets",
        "https://meets.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "ichiba",
        "リベシティ市場",
        "https://ichiba.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "furima",
        "リベシティフリーマーケット",
        "https://furima.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "space",
        "リベシティスペースシェアマーケット",
        "https://space.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "job",
        "リベシティジョブサーチ",
        "https://job.libecity.com/",
        "仕事・副業・売買",
        false,
    ),
    (
        "virtual-office",
        "リベシティバーチャルオフィス",
        "https://virtualoffice.libecity.com/mypage",
        "仕事・副業・売買",
        false,
    ),
];

// 外部サービスは内部WebViewのホスト許可リストに含めない。
const EXTERNAL_SERVICES: &[(&str, &str, &str, &str, bool)] = &[
    (
        "president-live",
        "学長ライブ",
        "https://www.youtube.com/playlist?list=PLpwLNivKud-h2NB97yKyezve0d0-0Lb2r",
        "学ぶ",
        false,
    ),
    (
        "city-live",
        "リベシティ限定ライブ",
        "https://www.youtube.com/playlist?list=PLpwLNivKud-ig9iaBUC8ZF4qa115h4sw1",
        "学ぶ",
        false,
    ),
];

/// サイドバー「すべてのサービス」のカテゴリ表示順
const CATEGORY_ORDER: &[&str] = &["本体", "学ぶ", "交流する", "仕事・副業・売買"];

/// リベシティのページへ注入する、お気に入りユーザーの★ボタン。
/// スクリプト側で origin を検査してから動く。
const FAVORITES_SCRIPT: &str = include_str!("inject/favorites.js");

fn service_url(service_id: &str) -> Option<&'static str> {
    SERVICES
        .iter()
        .find(|(id, _, _, _, _)| *id == service_id)
        .map(|(_, _, url, _, _)| *url)
}

fn has_userinfo(url: &Url) -> bool {
    !url.username().is_empty() || url.password().is_some()
}

/// サービス一覧に明示された HTTPS origin のみアプリ内 Webview で許可する。
fn is_allowed_internal_url(url: &Url) -> bool {
    if url.scheme() != "https" || has_userinfo(url) || url.port_or_known_default() != Some(443) {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };

    SERVICES.iter().any(|(_, _, service_url, _, _)| {
        Url::parse(service_url)
            .ok()
            .and_then(|service| service.host_str().map(|allowed| allowed == host))
            .unwrap_or(false)
    })
}

/// OS opener に渡してよい URL。未知のカスタム scheme と userinfo は拒否する。
fn is_allowed_external_url(url: &Url) -> bool {
    !has_userinfo(url) && matches!(url.scheme(), "https" | "http" | "mailto" | "tel")
}

fn request_external_open<R: Runtime>(app: &AppHandle<R>, url: &Url, prompt_open: Arc<AtomicBool>) {
    if !is_allowed_external_url(url) || prompt_open.swap(true, Ordering::AcqRel) {
        return;
    }

    let target = url.to_string();
    let display = {
        let mut value = url.clone();
        let _ = value.set_username("");
        let _ = value.set_password(None);
        value.set_query(None);
        value.set_fragment(None);
        value.to_string()
    };

    app.dialog()
        .message(format!(
            "外部ブラウザで次のリンクを開きますか？\n\n{display}"
        ))
        .title("外部リンクを開く")
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| {
            if approved {
                let _ = tauri_plugin_opener::open_url(&target, None::<&str>);
            }
            prompt_open.store(false, Ordering::Release);
        });
}

/// URL をログへ出す場合は資格情報・query・fragmentを必ず除去する。
#[cfg(debug_assertions)]
fn redact_url_for_log(url: &Url) -> String {
    let mut redacted = url.clone();
    let _ = redacted.set_username("");
    let _ = redacted.set_password(None);
    redacted.set_query(None);
    redacted.set_fragment(None);
    redacted.to_string()
}

#[derive(Clone, Serialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub category: String,
    pub pinned: bool,
    pub external: bool,
}

#[derive(Clone, Serialize)]
pub struct TabInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub active: bool,
}

#[derive(Clone)]
struct TabMeta {
    id: String,
    title: String,
    url: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
pub struct TabManagerState {
    tabs: Vec<TabMeta>,
    active_id: Option<String>,
    next_id: u64,
    content_bounds: Option<Bounds>,
}

pub type TabManager = Mutex<TabManagerState>;

fn snapshot(state: &TabManagerState) -> Vec<TabInfo> {
    state
        .tabs
        .iter()
        .map(|t| TabInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            url: t.url.clone(),
            active: state.active_id.as_deref() == Some(t.id.as_str()),
        })
        .collect()
}

/// Reorders the tab metadata without changing the active tab or any webviews.
/// Returns whether the tab order changed.
fn move_tab_inner(
    state: &mut TabManagerState,
    tab_id: &str,
    before_tab_id: Option<&str>,
) -> Result<bool, String> {
    let Some(source_index) = state.tabs.iter().position(|tab| tab.id == tab_id) else {
        return Err("tab not found".to_string());
    };

    if let Some(before_tab_id) = before_tab_id {
        if !state.tabs.iter().any(|tab| tab.id == before_tab_id) {
            return Err("before tab not found".to_string());
        }
        if before_tab_id == tab_id {
            return Ok(false);
        }
    }

    let tab = state.tabs.remove(source_index);
    let destination_index = match before_tab_id {
        Some(before_tab_id) => state
            .tabs
            .iter()
            .position(|candidate| candidate.id == before_tab_id)
            // The target was verified above and differs from the moved tab.
            .expect("validated before tab must remain after source removal"),
        None => state.tabs.len(),
    };
    state.tabs.insert(destination_index, tab);

    Ok(source_index != destination_index)
}

fn emit_tabs_changed<R: Runtime>(app: &AppHandle<R>, state: &TabManagerState) {
    let tabs = snapshot(state);
    let _ = app.emit("tabs-changed", &tabs);
    update_window_title(app, state);
    if let Err(err) = rebuild_app_menu(app, &tabs) {
        eprintln!("rebuild menu failed: {err}");
    }
}

fn update_window_title<R: Runtime>(app: &AppHandle<R>, _state: &TabManagerState) {
    // ウィンドウ一覧やOSの表示用タイトル。
    if let Some(window) = app.get_window("main") {
        let _ = window.set_title("Libe Desk");
    }
}

fn menu_tab_label(title: &str, url: &str) -> String {
    let raw = title.trim();
    let base = if raw.is_empty() || raw.starts_with("http://") || raw.starts_with("https://") {
        Url::parse(url)
            .ok()
            .and_then(|u| {
                u.host_str()
                    .map(|h| h.trim_start_matches("www.").to_string())
            })
            .unwrap_or_else(|| "ページ".to_string())
    } else {
        raw.to_string()
    };
    if base.chars().count() > 40 {
        format!("{}…", base.chars().take(39).collect::<String>())
    } else {
        base
    }
}

/// メニューバーの「タブ」メニューを現状のタブ一覧で作り直す。
pub fn rebuild_app_menu<R: Runtime>(app: &AppHandle<R>, tabs: &[TabInfo]) -> Result<(), String> {
    use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};

    let new_tab = MenuItemBuilder::with_id("tab-new", "新しいタブ")
        .accelerator("CmdOrCtrl+T")
        .build(app)
        .map_err(|e| e.to_string())?;
    let close_tab = MenuItemBuilder::with_id("tab-close", "タブを閉じる")
        .accelerator("CmdOrCtrl+W")
        .build(app)
        .map_err(|e| e.to_string())?;

    let mut tabs_menu = SubmenuBuilder::new(app, "タブ")
        .item(&new_tab)
        .item(&close_tab)
        .separator();

    for (i, tab) in tabs.iter().enumerate() {
        let id = format!("tab-switch:{}", tab.id);
        let label = menu_tab_label(&tab.title, &tab.url);
        let mut item = CheckMenuItemBuilder::with_id(&id, &label).checked(tab.active);
        if i < 9 {
            item = item.accelerator(format!("CmdOrCtrl+{}", i + 1));
        }
        let item = item.build(app).map_err(|e| e.to_string())?;
        tabs_menu = tabs_menu.item(&item);
    }
    let tabs_menu = tabs_menu.build().map_err(|e| e.to_string())?;

    let back = MenuItemBuilder::with_id("nav-back", "戻る")
        .accelerator("CmdOrCtrl+[")
        .build(app)
        .map_err(|e| e.to_string())?;
    let forward = MenuItemBuilder::with_id("nav-forward", "進む")
        .accelerator("CmdOrCtrl+]")
        .build(app)
        .map_err(|e| e.to_string())?;
    let reload = MenuItemBuilder::with_id("nav-reload", "再読み込み")
        .accelerator("CmdOrCtrl+R")
        .build(app)
        .map_err(|e| e.to_string())?;
    let navigate = SubmenuBuilder::new(app, "移動")
        .item(&back)
        .item(&forward)
        .item(&reload)
        .build()
        .map_err(|e| e.to_string())?;

    let edit = SubmenuBuilder::new(app, "編集")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()
        .map_err(|e| e.to_string())?;

    // macOSでは最初のサブメニューがアプリメニューになる。
    // 毎回の再構築でも標準の終了操作（Cmd+Q）を保持する。
    let application = SubmenuBuilder::new(app, "Libe Desk");
    #[cfg(target_os = "macos")]
    let application = application
        .hide_with_text("Libe Deskを隠す")
        .hide_others_with_text("ほかを隠す")
        .show_all_with_text("すべてを表示")
        .separator();
    let application = application
        .quit_with_text("Libe Deskを終了")
        .build()
        .map_err(|e| e.to_string())?;
    let window_menu = SubmenuBuilder::new(app, "ウィンドウ")
        .minimize_with_text("最小化")
        .maximize_with_text("拡大／縮小")
        .build()
        .map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(app)
        .item(&application)
        .item(&tabs_menu)
        .item(&navigate)
        .item(&edit)
        .item(&window_menu)
        .build()
        .map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    Ok(())
}

/// メニューからのタブ操作。
pub fn handle_tab_menu<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<(), String> {
    let state = app.state::<TabManager>();
    if id == "tab-new" {
        return open_service(app.clone(), state, "libecity".to_string()).map(|_| ());
    }
    if id == "tab-close" {
        let tab_id = active_tab_id(&state)?;
        return close_tab(app.clone(), state, tab_id);
    }
    if let Some(tab_id) = id.strip_prefix("tab-switch:") {
        return switch_tab(app.clone(), state, tab_id.to_string());
    }
    Ok(())
}

fn apply_visibility<R: Runtime>(app: &AppHandle<R>, state: &TabManagerState) {
    for tab in &state.tabs {
        if let Some(webview) = app.get_webview(&tab.id) {
            if state.active_id.as_deref() == Some(tab.id.as_str()) {
                let _ = webview.show();
                let _ = webview.set_focus();
            } else {
                let _ = webview.hide();
            }
        }
    }
}

const SIDEBAR_WIDTH: f64 = 220.0;
/// CSS の --window-drag-height + --titlebar-height と揃える。
const TITLEBAR_HEIGHT: f64 = 84.0;

#[cfg(debug_assertions)]
fn layout_log(message: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("libe-desk-layout.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{}", message);
    }
}

#[cfg(not(debug_assertions))]
fn layout_log(_message: &str) {}

fn window_logical_size<R: Runtime>(app: &AppHandle<R>) -> Option<(f64, f64)> {
    let window = app.get_window("main")?;
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().ok()?;
    Some((
        (size.width as f64 / scale).floor().max(1.0),
        (size.height as f64 / scale).floor().max(1.0),
    ))
}

fn compute_content_bounds(win_w: f64, win_h: f64) -> Bounds {
    Bounds {
        x: SIDEBAR_WIDTH,
        y: TITLEBAR_HEIGHT,
        width: (win_w - SIDEBAR_WIDTH).max(1.0),
        height: (win_h - TITLEBAR_HEIGHT).max(1.0),
    }
}

fn place_webview<R: Runtime>(
    webview: &tauri::Webview<R>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    label: &str,
) {
    if let Err(e) = webview.set_position(LogicalPosition::new(x, y)) {
        layout_log(&format!("{label} set_position err: {e}"));
    }
    if let Err(e) = webview.set_size(LogicalSize::new(width.max(1.0), height.max(1.0))) {
        layout_log(&format!("{label} set_size err: {e}"));
    }
    if let Ok(pos) = webview.position() {
        layout_log(&format!("{label} actual_pos={pos:?}"));
    }
}

/// メイン UI はウィンドウ全面（タイトルバータブ＋サイドバー）。
/// コンテンツはタイトルバー下・サイドバー右のみ。
pub fn apply_chrome_layout_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &mut TabManagerState,
) -> Result<(), String> {
    apply_chrome_layout_inner_with(app, state, false)
}

pub fn apply_chrome_layout_inner_with<R: Runtime>(
    app: &AppHandle<R>,
    state: &mut TabManagerState,
    _unused: bool,
) -> Result<(), String> {
    let (w, h) = window_logical_size(app).ok_or("cannot measure window")?;

    if let Some(tabs) = app.get_webview("ui-tabs") {
        let _ = tabs.close();
    }

    if let Some(ui) = app.get_webview("main") {
        place_webview(&ui, 0.0, 0.0, w, h, "main");
    }

    let bounds = compute_content_bounds(w, h);
    state.content_bounds = Some(bounds);

    for tab in &state.tabs {
        if let Some(webview) = app.get_webview(&tab.id) {
            place_webview(
                &webview,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                &tab.id,
            );
        }
    }

    layout_log(&format!(
        "layout w={w} h={h} content=({}, {}, {}, {})",
        bounds.x, bounds.y, bounds.width, bounds.height
    ));
    Ok(())
}

#[cfg(debug_assertions)]
fn smoke_log(message: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("libe-desk-smoke.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{}", message);
    }
}

fn spawn_tab_webview<R: Runtime>(
    app: &AppHandle<R>,
    tab_id: String,
    url: Url,
    bounds: Bounds,
) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window not found")?;

    let nav_app = app.clone();
    let nav_tab_id = tab_id.clone();
    let nav_prompt_open = Arc::new(AtomicBool::new(false));
    let title_app = app.clone();
    let title_tab_id = tab_id.clone();

    #[cfg(debug_assertions)]
    smoke_log(&format!(
        "spawn_tab id={tab_id} url={} bounds=({}, {}, {}, {})",
        redact_url_for_log(&url),
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height
    ));

    let builder = WebviewBuilder::new(tab_id.clone(), WebviewUrl::External(url))
        .initialization_script(FAVORITES_SCRIPT)
        .on_navigation(move |nav_url| {
            if is_allowed_internal_url(nav_url) {
                let state = nav_app.state::<TabManager>();
                let mut guard = state.lock().unwrap();
                if let Some(t) = guard.tabs.iter_mut().find(|t| t.id == nav_tab_id) {
                    t.url = nav_url.to_string();
                }
                emit_tabs_changed(&nav_app, &guard);
                true
            } else {
                // 内部許可外の遷移はWebViewでは拒否し、許可schemeだけ
                // ネイティブ確認ダイアログを経て外部ブラウザへ渡す。
                #[cfg(debug_assertions)]
                smoke_log(&format!(
                    "blocked_navigation {}",
                    redact_url_for_log(nav_url)
                ));
                request_external_open(&nav_app, nav_url, nav_prompt_open.clone());
                false
            }
        })
        .on_document_title_changed(move |_webview, title| {
            let state = title_app.state::<TabManager>();
            let mut guard = state.lock().unwrap();
            if let Some(t) = guard.tabs.iter_mut().find(|t| t.id == title_tab_id) {
                t.title = title;
            }
            emit_tabs_changed(&title_app, &guard);
        });

    window
        .add_child(
            builder,
            LogicalPosition::new(bounds.x.floor(), bounds.y.floor()),
            LogicalSize::new(
                bounds.width.floor().max(1.0),
                bounds.height.floor().max(1.0),
            ),
        )
        .map_err(|e| e.to_string())?;

    #[cfg(debug_assertions)]
    smoke_log(&format!("spawn_ok id={tab_id}"));

    Ok(())
}

fn open_url_internal<R: Runtime>(
    app: &AppHandle<R>,
    state: &tauri::State<'_, TabManager>,
    raw_url: &str,
) -> Result<TabInfo, String> {
    open_url_internal_with_title(app, state, raw_url, None)
}

pub(crate) fn open_url_internal_with_title<R: Runtime>(
    app: &AppHandle<R>,
    state: &tauri::State<'_, TabManager>,
    raw_url: &str,
    initial_title: Option<&str>,
) -> Result<TabInfo, String> {
    let parsed = Url::parse(raw_url).map_err(|e| e.to_string())?;

    if !is_allowed_internal_url(&parsed) {
        return Err("URL is not an allowed internal service".to_string());
    }

    // レイアウト確定（サイドバー / タブ帯 / コンテンツ枠）
    {
        let mut guard = state.lock().unwrap();
        apply_chrome_layout_inner_with(app, &mut guard, true)?;
    }

    let bounds = {
        let guard = state.lock().unwrap();
        guard
            .content_bounds
            .ok_or_else(|| "content bounds missing after layout".to_string())?
    };

    let tab_id = {
        let mut guard = state.lock().unwrap();
        guard.next_id += 1;
        format!("tab-{}", guard.next_id)
    };

    spawn_tab_webview(app, tab_id.clone(), parsed.clone(), bounds)?;

    let title = initial_title
        .map(|s| s.to_string())
        .unwrap_or_else(|| parsed.host_str().unwrap_or("ページ").to_string());

    let mut guard = state.lock().unwrap();
    guard.tabs.push(TabMeta {
        id: tab_id.clone(),
        title,
        url: parsed.as_str().to_string(),
    });
    guard.active_id = Some(tab_id.clone());
    apply_visibility(app, &guard);
    // 新規コンテンツ Webview の上にタブ帯を載せ直す
    apply_chrome_layout_inner_with(app, &mut guard, true)?;
    emit_tabs_changed(app, &guard);

    snapshot(&guard)
        .into_iter()
        .find(|t| t.id == tab_id)
        .ok_or_else(|| "failed to create tab".to_string())
}

#[tauri::command]
pub fn list_services() -> Vec<ServiceInfo> {
    let mut services: Vec<ServiceInfo> = SERVICES
        .iter()
        .chain(EXTERNAL_SERVICES.iter())
        .map(|(id, name, url, category, pinned)| ServiceInfo {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            category: category.to_string(),
            pinned: *pinned,
            external: EXTERNAL_SERVICES
                .iter()
                .any(|(external_id, ..)| external_id == id),
        })
        .collect();

    services.sort_by(|a, b| {
        let ai = CATEGORY_ORDER
            .iter()
            .position(|c| *c == a.category.as_str())
            .unwrap_or(usize::MAX);
        let bi = CATEGORY_ORDER
            .iter()
            .position(|c| *c == b.category.as_str())
            .unwrap_or(usize::MAX);
        ai.cmp(&bi)
    });

    services
}

#[tauri::command]
pub fn open_service<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    service_id: String,
) -> Result<Option<TabInfo>, String> {
    if let Some((_, _, raw_url, _, _)) = EXTERNAL_SERVICES.iter().find(|(id, ..)| *id == service_id)
    {
        let url = Url::parse(raw_url).map_err(|e| e.to_string())?;
        request_external_open(&app, &url, Arc::new(AtomicBool::new(false)));
        return Ok(None);
    }
    let (name, url) = SERVICES
        .iter()
        .find(|(id, _, _, _, _)| *id == service_id)
        .map(|(_, name, url, _, _)| (*name, *url))
        .ok_or("unknown service")?;
    open_url_internal_with_title(&app, &state, url, Some(name)).map(Some)
}

#[tauri::command]
pub fn list_tabs(state: tauri::State<'_, TabManager>) -> Vec<TabInfo> {
    let guard = state.lock().unwrap();
    snapshot(&guard)
}

#[tauri::command]
pub fn switch_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    if !guard.tabs.iter().any(|t| t.id == tab_id) {
        return Err("tab not found".into());
    }
    guard.active_id = Some(tab_id.clone());
    #[cfg(debug_assertions)]
    smoke_log(&format!("switch_tab {tab_id}"));
    apply_visibility(&app, &guard);
    emit_tabs_changed(&app, &guard);
    Ok(())
}

#[tauri::command]
pub fn move_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
    before_tab_id: Option<String>,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    if move_tab_inner(&mut guard, &tab_id, before_tab_id.as_deref())? {
        emit_tabs_changed(&app, &guard);
    }
    Ok(())
}

#[tauri::command]
pub fn close_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    smoke_log(&format!("close_tab {tab_id}"));
    let is_empty = {
        let mut guard = state.lock().unwrap();
        let Some(index) = guard.tabs.iter().position(|t| t.id == tab_id) else {
            return Ok(());
        };

        if let Some(webview) = app.get_webview(&tab_id) {
            let _ = webview.close();
        }
        guard.tabs.remove(index);

        if guard.active_id.as_deref() == Some(tab_id.as_str()) {
            let fallback_index = index.min(guard.tabs.len().saturating_sub(1));
            guard.active_id = guard.tabs.get(fallback_index).map(|t| t.id.clone());
        }

        apply_visibility(&app, &guard);
        emit_tabs_changed(&app, &guard);
        guard.tabs.is_empty()
    };

    // 最後のタブを閉じた場合は、常にリベシティ本体を開き直す。
    if is_empty {
        open_url_internal(&app, &state, service_url("libecity").unwrap())?;
    }

    Ok(())
}

#[tauri::command]
pub fn go_back<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    #[cfg(debug_assertions)]
    smoke_log(&format!("go_back {tab_id}"));
    let webview = app.get_webview(&tab_id).ok_or("tab not found")?;
    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn go_forward<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    #[cfg(debug_assertions)]
    smoke_log(&format!("go_forward {tab_id}"));
    let webview = app.get_webview(&tab_id).ok_or("tab not found")?;
    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reload_tab<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    #[cfg(debug_assertions)]
    smoke_log(&format!("reload_tab {tab_id}"));
    let webview = app.get_webview(&tab_id).ok_or("tab not found")?;
    webview.reload().map_err(|e| e.to_string())
}

fn active_tab_id(state: &tauri::State<'_, TabManager>) -> Result<String, String> {
    state
        .lock()
        .unwrap()
        .active_id
        .clone()
        .ok_or_else(|| "no active tab".to_string())
}

/// メニューバー等から、現在のアクティブタブに対して操作する。
pub fn navigate_active<R: Runtime>(app: &AppHandle<R>, action: &str) -> Result<(), String> {
    let state = app.state::<TabManager>();
    let tab_id = active_tab_id(&state)?;
    match action {
        "back" => go_back(app.clone(), tab_id),
        "forward" => go_forward(app.clone(), tab_id),
        "reload" => reload_tab(app.clone(), tab_id),
        _ => Err("unknown navigation action".into()),
    }
}

#[tauri::command]
pub fn get_current_page(
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<TabInfo, String> {
    let guard = state.lock().unwrap();
    snapshot(&guard)
        .into_iter()
        .find(|t| t.id == tab_id)
        .ok_or_else(|| "tab not found".to_string())
}

#[tauri::command]
pub fn apply_chrome_layout<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    apply_chrome_layout_inner_with(&app, &mut guard, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab_state(ids: &[&str], active_id: &str) -> TabManagerState {
        TabManagerState {
            tabs: ids
                .iter()
                .map(|id| TabMeta {
                    id: (*id).to_string(),
                    title: format!("title-{id}"),
                    url: format!("https://example.com/{id}"),
                })
                .collect(),
            active_id: Some(active_id.to_string()),
            ..Default::default()
        }
    }

    fn tab_ids(state: &TabManagerState) -> Vec<&str> {
        state.tabs.iter().map(|tab| tab.id.as_str()).collect()
    }

    #[test]
    fn move_tab_reorders_forward_without_changing_active_tab() {
        let mut state = tab_state(&["a", "b", "c", "d"], "c");

        assert_eq!(move_tab_inner(&mut state, "a", Some("d")), Ok(true));

        assert_eq!(tab_ids(&state), ["b", "c", "a", "d"]);
        assert_eq!(state.active_id.as_deref(), Some("c"));
    }

    #[test]
    fn move_tab_reorders_backward() {
        let mut state = tab_state(&["a", "b", "c", "d"], "d");

        assert_eq!(move_tab_inner(&mut state, "d", Some("b")), Ok(true));

        assert_eq!(tab_ids(&state), ["a", "d", "b", "c"]);
        assert_eq!(state.active_id.as_deref(), Some("d"));
    }

    #[test]
    fn move_tab_moves_to_end() {
        let mut state = tab_state(&["a", "b", "c"], "b");

        assert_eq!(move_tab_inner(&mut state, "a", None), Ok(true));

        assert_eq!(tab_ids(&state), ["b", "c", "a"]);
        assert_eq!(state.active_id.as_deref(), Some("b"));
    }

    #[test]
    fn move_tab_rejects_unknown_ids_without_mutating_state() {
        let mut state = tab_state(&["a", "b", "c"], "b");

        assert_eq!(
            move_tab_inner(&mut state, "missing", Some("b")),
            Err("tab not found".into())
        );
        assert_eq!(
            move_tab_inner(&mut state, "a", Some("missing")),
            Err("before tab not found".into())
        );

        assert_eq!(tab_ids(&state), ["a", "b", "c"]);
        assert_eq!(state.active_id.as_deref(), Some("b"));
    }

    #[test]
    fn move_tab_to_itself_is_a_noop() {
        let mut state = tab_state(&["a", "b", "c"], "b");

        assert_eq!(move_tab_inner(&mut state, "b", Some("b")), Ok(false));

        assert_eq!(tab_ids(&state), ["a", "b", "c"]);
        assert_eq!(state.active_id.as_deref(), Some("b"));
    }

    #[test]
    fn allowed_internal_urls_require_exact_https_service_hosts() {
        assert!(is_allowed_internal_url(
            &Url::parse("https://libecity.com/room_list").unwrap()
        ));
        assert!(is_allowed_internal_url(
            &Url::parse("https://library.libecity.com/").unwrap()
        ));
        assert!(is_allowed_internal_url(
            &Url::parse("https://libecity.com:443/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("http://libecity.com/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("https://unknown.libecity.com/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("https://evil-libecity.com/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("https://user:secret@libecity.com/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("https://libecity.com:8443/").unwrap()
        ));
    }

    #[test]
    fn external_urls_allow_only_safe_schemes_without_userinfo() {
        for url in [
            "https://example.com/",
            "http://example.com/",
            "mailto:test@example.com",
            "tel:+81000000000",
        ] {
            assert!(is_allowed_external_url(&Url::parse(url).unwrap()), "{url}");
        }

        for url in [
            "file:///tmp/example",
            "data:text/plain,example",
            "javascript:alert(1)",
            "custom-app://open/something",
            "https://user:secret@example.com/",
        ] {
            assert!(!is_allowed_external_url(&Url::parse(url).unwrap()), "{url}");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn logged_urls_drop_credentials_query_and_fragment() {
        let url =
            Url::parse("https://user:secret@libecity.com/path?token=secret#fragment").unwrap();
        let logged = redact_url_for_log(&url);
        assert_eq!(logged, "https://libecity.com/path");
    }

    #[test]
    fn external_services_do_not_expand_internal_webview_permissions() {
        for (_, _, raw_url, _, _) in EXTERNAL_SERVICES {
            let url = Url::parse(raw_url).unwrap();
            assert!(is_allowed_external_url(&url));
            assert!(!is_allowed_internal_url(&url));
        }
        assert!(is_allowed_internal_url(
            &Url::parse("https://lifeplan.libecity.com/").unwrap()
        ));
        assert!(!is_allowed_internal_url(
            &Url::parse("https://lifeplan.libecity.com.evil.example/").unwrap()
        ));
    }

    #[test]
    fn service_ids_remain_unique_across_internal_and_external_menus() {
        let services = list_services();
        let mut ids: Vec<_> = services.iter().map(|service| &service.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), services.len());
        assert_eq!(
            services.iter().filter(|service| service.external).count(),
            2
        );
    }

    #[test]
    fn service_urls() {
        assert_eq!(service_url("libecity"), Some("https://libecity.com/"));
        assert_eq!(
            service_url("library"),
            Some("https://library.libecity.com/")
        );
        assert_eq!(service_url("works"), Some("https://works.libecity.com/"));
        assert_eq!(service_url("unknown"), None);
    }

    #[test]
    fn default_pinned_only_libecity() {
        let pinned: Vec<_> = SERVICES.iter().filter(|(_, _, _, _, p)| *p).collect();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].0, "libecity");
    }

    #[test]
    fn categories_cover_order() {
        for cat in CATEGORY_ORDER {
            assert!(
                SERVICES.iter().any(|(_, _, _, c, _)| c == cat),
                "missing category {cat}"
            );
        }
    }

    #[test]
    fn no_duplicate_urls() {
        let mut urls: Vec<&str> = SERVICES.iter().map(|(_, _, url, _, _)| *url).collect();
        urls.sort_unstable();
        urls.dedup();
        assert_eq!(urls.len(), SERVICES.len());
    }
}

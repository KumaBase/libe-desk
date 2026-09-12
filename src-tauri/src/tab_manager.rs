//! タブ・Webview操作の共通コントローラ層。
//!
//! UI(SolidJS)はここで定義された tauri コマンドのみを呼び出し、
//! Webviewの生成・表示切替・ナビゲーション制御の詳細は知らない。
//! 将来 MCP サーバーを追加する場合も、同じ関数群を呼び出すアダプタを
//! 追加するだけで済むようにするための分離。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, Url, WebviewUrl,
};

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

/// 外部ページもWebViewで表示できるが、アプリのIPC権限は付与しない。
fn is_allowed_web_url(url: &Url) -> bool {
    !has_userinfo(url) && matches!(url.scheme(), "https" | "http") && url.host_str().is_some()
}

#[derive(Debug, PartialEq)]
enum NavigationAction {
    CurrentTab,
    NewTab,
    SystemApp,
    Block,
}

fn navigation_action(url: &Url, external_tab: bool) -> NavigationAction {
    if is_allowed_web_url(url) {
        // 外部タブ内の遷移・リダイレクトは同じタブで継続する。
        if external_tab || is_allowed_internal_url(url) {
            NavigationAction::CurrentTab
        } else {
            NavigationAction::NewTab
        }
    } else if !has_userinfo(url) && matches!(url.scheme(), "mailto" | "tel") {
        NavigationAction::SystemApp
    } else {
        NavigationAction::Block
    }
}

fn open_link_in_new_tab<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if !is_allowed_web_url(url) {
        open_system_link(url);
        return;
    }
    let app = app.clone();
    let target = url.to_string();
    // WebViewのコールバック内で同期的に別WebViewを作成しない。
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TabManager>();
        if let Err(err) = open_url_internal_with_title(&app, &state, &target, None) {
            eprintln!("open link tab failed: {err}");
        }
    });
}

fn open_system_link(url: &Url) {
    if !has_userinfo(url) && matches!(url.scheme(), "mailto" | "tel") {
        let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
    }
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
    applied_bounds: Option<Bounds>,
    /// ページの読み込みが開始済みか（Windowsの停滞検出用）。
    /// 読み取りはWindows限定のため、他OSでは未読み込みフィールドとして
    /// dead_code 警告を出さない。
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    load_started: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    menu_tabs: Vec<TabInfo>,
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

fn emit_tabs_changed<R: Runtime>(app: &AppHandle<R>, state: &mut TabManagerState) {
    let tabs = snapshot(state);
    let _ = app.emit("tabs-changed", &tabs);
    match update_app_menu(app, &state.menu_tabs, &tabs) {
        Ok(()) => state.menu_tabs = tabs,
        Err(err) => eprintln!("update menu failed: {err}"),
    }
}

fn menu_structure_changed(previous: &[TabInfo], tabs: &[TabInfo]) -> bool {
    !previous
        .iter()
        .map(|tab| &tab.id)
        .eq(tabs.iter().map(|tab| &tab.id))
}

fn update_app_menu<R: Runtime>(
    app: &AppHandle<R>,
    previous: &[TabInfo],
    tabs: &[TabInfo],
) -> Result<(), String> {
    if menu_structure_changed(previous, tabs) {
        return rebuild_app_menu(app, tabs);
    }
    // URLだけの変更など、表示に影響しない通知ではOSへの呼び出しも省く。
    let changed: Vec<_> = previous
        .iter()
        .zip(tabs)
        .filter(|(old, new)| {
            old.active != new.active
                || menu_tab_label(&old.title, &old.url) != menu_tab_label(&new.title, &new.url)
        })
        .collect();
    if changed.is_empty() {
        return Ok(());
    }
    let menu = app.menu().ok_or("app menu missing")?;
    let item = menu.get("tabs-menu").ok_or("tabs menu missing")?;
    let submenu = item.as_submenu().ok_or("invalid tabs menu")?;
    for (old, tab) in changed {
        let item = submenu
            .get(&format!("tab-switch:{}", tab.id))
            .ok_or("tab menu item missing")?;
        let item = item.as_check_menuitem().ok_or("invalid tab menu item")?;
        let label = menu_tab_label(&tab.title, &tab.url);
        if menu_tab_label(&old.title, &old.url) != label {
            item.set_text(label).map_err(|e| e.to_string())?;
        }
        if old.active != tab.active {
            item.set_checked(tab.active).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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

    let mut tabs_menu = SubmenuBuilder::with_id(app, "tabs-menu", "タブ")
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

/// メニューからのタブ操作。イベントハンドラはメインスレッドで動くため、
/// WebViewを生成し得る操作は spawn_blocking に回す（open_link_in_new_tab と同じ理由）。
pub fn handle_tab_menu<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<(), String> {
    if id == "tab-new" {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<TabManager>();
            if let Err(err) = open_service_impl(app.clone(), state, "libecity".to_string(), None) {
                eprintln!("tab-new failed: {err}");
            }
        });
        return Ok(());
    }
    if id == "tab-close" {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<TabManager>();
            // ロック取得はメインスレッド外で行う（デッドロック防止）。
            let Ok(tab_id) = active_tab_id(&state) else {
                return;
            };
            if let Err(err) = close_tab_impl(app.clone(), state, tab_id) {
                eprintln!("tab-close failed: {err}");
            }
        });
        return Ok(());
    }
    if let Some(tab_id) = id.strip_prefix("tab-switch:") {
        let tab_id = tab_id.to_string();
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<TabManager>();
            if let Err(err) = switch_tab_impl(app.clone(), state, tab_id) {
                eprintln!("tab-switch failed: {err}");
            }
        });
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
) -> Result<(), String> {
    if let Err(e) = webview.set_position(LogicalPosition::new(x, y)) {
        return Err(format!("{label} set_position err: {e}"));
    }
    if let Err(e) = webview.set_size(LogicalSize::new(width.max(1.0), height.max(1.0))) {
        return Err(format!("{label} set_size err: {e}"));
    }
    #[cfg(debug_assertions)]
    if let Ok(pos) = webview.position() {
        layout_log(&format!("{label} actual_pos={pos:?}"));
    }
    Ok(())
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

    let bounds = compute_content_bounds(w, h);
    if state.content_bounds != Some(bounds) {
        if let Some(ui) = app.get_webview("main") {
            place_webview(&ui, 0.0, 0.0, w, h, "main")?;
        }
        state.content_bounds = Some(bounds);
    }

    // 非表示タブのサイズ変更は、表示直前まで遅延させる。
    for tab in &mut state.tabs {
        if state.active_id.as_deref() != Some(tab.id.as_str()) || tab.applied_bounds == Some(bounds)
        {
            continue;
        }
        if let Some(webview) = app.get_webview(&tab.id) {
            place_webview(
                &webview,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                &tab.id,
            )?;
            tab.applied_bounds = Some(bounds);
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

/// WebView2のイベントコールバック（メインスレッド）からタブ状態を更新する。
/// ロック取得と tabs-changed の通知を別スレッドで行い、メインスレッドを
/// ロック待ちで止めない（デッドロック防止）。クロージャは変更があったか
/// を返し、変わっていなければ通知しない。タブが見つからない場合は何もしない。
fn update_tab_state<R, F>(app: AppHandle<R>, tab_id: String, update: F)
where
    R: Runtime,
    F: FnOnce(&mut TabMeta) -> bool + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TabManager>();
        let mut guard = state.lock().unwrap();
        let Some(t) = guard.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        if !update(t) {
            return;
        }
        emit_tabs_changed(&app, &mut guard);
    });
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
    let external_tab = !is_allowed_internal_url(&url);
    let popup_app = app.clone();
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
        .on_page_load(|webview, event| {
            // WindowsのWebView2で読み込みが停滞した場合、この Started が
            // 一切来ない。停滞検出の判定に使うため記録する。
            if matches!(event.event(), PageLoadEvent::Started) {
                let app = webview.app_handle().clone();
                let tab_id = webview.label().to_string();
                update_tab_state(app, tab_id, |t| {
                    if t.load_started {
                        return false;
                    }
                    t.load_started = true;
                    true
                });
            }
        })
        .on_navigation(move |nav_url| {
            // このコールバックはメインスレッドで実行される。ここで
            // TabManager のロックを同期的に取ると、ロックを保持したまま
            // メインスレッドを待つワーカーとデッドロックするため、
            // 判定だけ同期的に行い、記録は別スレッドで行う。
            match navigation_action(nav_url, external_tab) {
                NavigationAction::CurrentTab => {
                    let app = nav_app.clone();
                    let tab_id = nav_tab_id.clone();
                    let url = nav_url.to_string();
                    update_tab_state(app, tab_id, move |t| {
                        if t.url == url {
                            return false;
                        }
                        t.url = url;
                        true
                    });
                    true
                }
                NavigationAction::NewTab => {
                    open_link_in_new_tab(&nav_app, nav_url);
                    false
                }
                NavigationAction::SystemApp => {
                    open_system_link(nav_url);
                    false
                }
                NavigationAction::Block => false,
            }
        })
        .on_new_window(move |url, _features| {
            open_link_in_new_tab(&popup_app, &url);
            tauri::webview::NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_webview, title| {
            let app = title_app.clone();
            let tab_id = title_tab_id.clone();
            update_tab_state(app, tab_id, move |t| {
                if t.title == title {
                    return false;
                }
                t.title = title;
                true
            });
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

    // WindowsのWebView2では、生成直後の初期ナビゲーションが
    // NavigationStarting で止まったまま完了しないことがある
    // （https://github.com/tauri-apps/tauri/issues/10011）。
    // 読み込み開始（on_page_load Started）が確認できない場合に限り、
    // 遅延して reload を試す。未コミットの初期読み込みへの即時 reload は
    // 意図しないURLへ遷移することがあるため、即実行はしない。
    #[cfg(target_os = "windows")]
    {
        let app = app.clone();
        let nudge_tab_id = tab_id.clone();
        std::thread::spawn(move || {
            for delay_ms in [1000u64, 4000] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                let stalled = {
                    let state = app.state::<TabManager>();
                    let guard = state.lock().unwrap();
                    !guard
                        .tabs
                        .iter()
                        .any(|t| t.id == nudge_tab_id && t.load_started)
                };
                if !stalled {
                    return;
                }
                if let Some(webview) = app.get_webview(&nudge_tab_id) {
                    let _ = webview.reload();
                }
            }
        });
    }

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

    if !is_allowed_web_url(&parsed) {
        return Err("URL is not an allowed web page".to_string());
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
        applied_bounds: Some(bounds),
        load_started: false,
    });
    guard.active_id = Some(tab_id.clone());
    apply_visibility(app, &guard);
    // 生成中にウィンドウサイズが変わった場合も最新のサイズに合わせる
    apply_chrome_layout_inner_with(app, &mut guard, true)?;
    emit_tabs_changed(app, &mut guard);

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

/// open_service の実体。WebViewの生成を伴うため、Windowsでは
/// メインスレッド（同期コマンド・イベントハンドラ）から呼べない。
/// 呼び出し側は async コマンドか spawn_blocking 上で実行すること。
pub fn open_service_impl<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    service_id: String,
    reuse_existing: Option<bool>,
) -> Result<Option<TabInfo>, String> {
    if let Some((_, name, raw_url, _, _)) =
        EXTERNAL_SERVICES.iter().find(|(id, ..)| *id == service_id)
    {
        return open_url_internal_with_title(&app, &state, raw_url, Some(name)).map(Some);
    }
    let (name, url) = SERVICES
        .iter()
        .find(|(id, _, _, _, _)| *id == service_id)
        .map(|(_, name, url, _, _)| (*name, *url))
        .ok_or("unknown service")?;
    if reuse_existing.unwrap_or(false) {
        if let Some(tab) = reuse_url(&app, &state, url)? {
            return Ok(Some(tab));
        }
    }
    open_url_internal_with_title(&app, &state, url, Some(name)).map(Some)
}

/// WebView2の制約で、同期コマンド内のWebView生成はデッドロック・白表示化する。
/// そのため WebView を作り得るコマンドは async にして実行スレッドをずらす。
/// https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html#known-issues
#[tauri::command]
pub async fn open_service<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    service_id: String,
    reuse_existing: Option<bool>,
) -> Result<Option<TabInfo>, String> {
    open_service_impl(app, state, service_id, reuse_existing)
}

/// サイドバーの同一URLだけ再利用する。新規タブ操作には適用しない。
pub(crate) fn reuse_url<R: Runtime>(
    app: &AppHandle<R>,
    state: &tauri::State<'_, TabManager>,
    url: &str,
) -> Result<Option<TabInfo>, String> {
    let ids: Vec<_> = {
        let guard = state.lock().unwrap();
        guard.tabs.iter().map(|tab| tab.id.clone()).collect()
    };
    // SPAの履歴変更はナビゲーション通知が来ない場合があるため、
    // 保存済みURLではなくWebViewの現在地を照合する。
    let id = ids.into_iter().find(|id| {
        app.get_webview(id)
            .and_then(|view| view.url().ok())
            .is_some_and(|current| current.as_str() == url)
    });
    let Some(id) = id else {
        return Ok(None);
    };
    switch_tab_impl(app.clone(), state.clone(), id.clone())?;
    let guard = state.lock().unwrap();
    Ok(snapshot(&guard).into_iter().find(|tab| tab.id == id))
}

/// 状態の読み取りのみだが、同期コマンドはメインスレッドで実行され、
/// ロック待ちでメインスレッドを止めうるため async にする。
#[tauri::command]
pub async fn list_tabs(state: tauri::State<'_, TabManager>) -> Result<Vec<TabInfo>, String> {
    let guard = state.lock().unwrap();
    Ok(snapshot(&guard))
}

/// switch_tab の実体。WebViewの表示切替を伴うため、メインスレッド
/// 以外（async コマンド・spawn_blocking）から呼ぶこと。
pub fn switch_tab_impl<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    if !guard.tabs.iter().any(|t| t.id == tab_id) {
        return Err("tab not found".into());
    }
    if guard.active_id.as_deref() == Some(tab_id.as_str()) {
        if let Some(webview) = app.get_webview(&tab_id) {
            let _ = webview.set_focus();
        }
        return Ok(());
    }
    guard.active_id = Some(tab_id.clone());
    apply_chrome_layout_inner(&app, &mut guard)?;
    #[cfg(debug_assertions)]
    smoke_log(&format!("switch_tab {tab_id}"));
    apply_visibility(&app, &guard);
    emit_tabs_changed(&app, &mut guard);
    Ok(())
}

#[tauri::command]
pub async fn switch_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<(), String> {
    switch_tab_impl(app, state, tab_id)
}

/// move_tab の実体。メニュー再構築を伴うため switch_tab_impl と同じ扱い。
pub fn move_tab_impl<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
    before_tab_id: Option<String>,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    if move_tab_inner(&mut guard, &tab_id, before_tab_id.as_deref())? {
        emit_tabs_changed(&app, &mut guard);
    }
    Ok(())
}

#[tauri::command]
pub async fn move_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
    before_tab_id: Option<String>,
) -> Result<(), String> {
    move_tab_impl(app, state, tab_id, before_tab_id)
}

/// close_tab の実体。最後のタブを閉じた際にWebViewを作り直すため、
/// open_service_impl と同様にメインスレッド以外から呼ぶこと。
pub fn close_tab_impl<R: Runtime>(
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

        apply_chrome_layout_inner(&app, &mut guard)?;
        apply_visibility(&app, &guard);
        emit_tabs_changed(&app, &mut guard);
        guard.tabs.is_empty()
    };

    // 最後のタブを閉じた場合は、常にリベシティ本体を開き直す。
    if is_empty {
        open_url_internal(&app, &state, service_url("libecity").unwrap())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn close_tab<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, TabManager>,
    tab_id: String,
) -> Result<(), String> {
    close_tab_impl(app, state, tab_id)
}

/// go_back / go_forward / reload_tab の実体。WebViewへの操作を
/// ディスパッチするため、メインスレッド以外から呼ぶこと。
pub fn navigate_impl<R: Runtime>(
    app: &AppHandle<R>,
    action: &str,
    tab_id: &str,
) -> Result<(), String> {
    let webview = app.get_webview(tab_id).ok_or("tab not found")?;
    match action {
        "back" => {
            #[cfg(debug_assertions)]
            smoke_log(&format!("go_back {tab_id}"));
            webview
                .eval("window.history.back()")
                .map_err(|e| e.to_string())
        }
        "forward" => {
            #[cfg(debug_assertions)]
            smoke_log(&format!("go_forward {tab_id}"));
            webview
                .eval("window.history.forward()")
                .map_err(|e| e.to_string())
        }
        "reload" => {
            #[cfg(debug_assertions)]
            smoke_log(&format!("reload_tab {tab_id}"));
            webview.reload().map_err(|e| e.to_string())
        }
        _ => Err("unknown navigation action".into()),
    }
}

#[tauri::command]
pub async fn go_back<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    navigate_impl(&app, "back", &tab_id)
}

#[tauri::command]
pub async fn go_forward<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    navigate_impl(&app, "forward", &tab_id)
}

#[tauri::command]
pub async fn reload_tab<R: Runtime>(app: AppHandle<R>, tab_id: String) -> Result<(), String> {
    navigate_impl(&app, "reload", &tab_id)
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
/// メニューのイベントハンドラはメインスレッドで動くため、
/// spawn_blocking 経由で呼ぶこと。
pub fn navigate_active<R: Runtime>(app: &AppHandle<R>, action: &str) -> Result<(), String> {
    let state = app.state::<TabManager>();
    let tab_id = active_tab_id(&state)?;
    navigate_impl(app, action, &tab_id)
}

#[tauri::command]
pub async fn get_current_page(
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
pub async fn apply_chrome_layout<R: Runtime>(
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
                    applied_bounds: None,
                    load_started: true,
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
    fn menu_rebuild_only_for_tab_structure_changes() {
        let original = snapshot(&tab_state(&["a", "b"], "a"));
        let mut changed = original.clone();
        changed[0].title = "new title".into();
        changed[0].url = "https://libecity.com/room_list".into();
        changed[0].active = false;
        changed[1].active = true;
        assert!(!menu_structure_changed(&original, &changed));
        changed.swap(0, 1);
        assert!(menu_structure_changed(&original, &changed));
        assert!(menu_structure_changed(&original, &original[..1]));
        assert!(menu_structure_changed(&[], &original));
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
    fn web_urls_allow_only_http_and_https_without_userinfo() {
        for url in ["https://example.com/", "http://example.com/"] {
            assert!(is_allowed_web_url(&Url::parse(url).unwrap()), "{url}");
        }

        for url in [
            "mailto:test@example.com",
            "tel:+81000000000",
            "file:///tmp/example",
            "data:text/plain,example",
            "javascript:alert(1)",
            "custom-app://open/something",
            "https://user:secret@example.com/",
        ] {
            assert!(!is_allowed_web_url(&Url::parse(url).unwrap()), "{url}");
        }
    }

    #[test]
    fn external_links_open_new_tabs_without_redirect_loops() {
        let external = Url::parse("https://example.com/page").unwrap();
        let internal = Url::parse("https://libecity.com/room_list").unwrap();
        assert_eq!(
            navigation_action(&external, false),
            NavigationAction::NewTab
        );
        assert_eq!(
            navigation_action(&external, true),
            NavigationAction::CurrentTab
        );
        assert_eq!(
            navigation_action(&internal, false),
            NavigationAction::CurrentTab
        );
        assert_eq!(
            navigation_action(&internal, true),
            NavigationAction::CurrentTab
        );
        assert_eq!(
            navigation_action(&Url::parse("mailto:test@example.com").unwrap(), false),
            NavigationAction::SystemApp
        );
        for target in [
            "javascript:alert(1)",
            "file:///tmp/test",
            "data:text/html,test",
            "https://user:pass@example.com/",
        ] {
            assert_eq!(
                navigation_action(&Url::parse(target).unwrap(), true),
                NavigationAction::Block
            );
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
            assert!(is_allowed_web_url(&url));
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

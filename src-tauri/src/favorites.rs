//! お気に入りユーザー（リベシティのプロフィールページへのローカルブックマーク）。
//!
//! リベシティのページへ注入したスクリプトと、アプリ側のサイドバーの双方が
//! 同じ一覧を読み書きするため、保存の実体はフロントの localStorage ではなく
//! ここ（Rust 側の JSON ファイル）に置く。
//!
//! 注入スクリプトはリベシティのページ上で動くため、そこから渡る値は
//! 信用せずすべて検証・正規化してから保持する。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// プロフィール URL に埋め込めるユーザー ID の最大長
const MAX_ID_LEN: usize = 64;
/// 表示名の最大文字数
const MAX_NAME_CHARS: usize = 60;
/// 登録できるお気に入りの上限
const MAX_FAVORITES: usize = 200;

const STORE_FILE: &str = "favorites.json";
const PROFILE_URL_PREFIX: &str = "https://libecity.com/user_profile/";

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteUser {
    pub id: String,
    pub name: String,
    pub added_at: u64,
}

pub type FavoriteStore = Mutex<Vec<FavoriteUser>>;

/// リベシティのユーザー ID として受け入れてよい形だけを通す。
fn normalize_id(raw: &str) -> Option<String> {
    let id = raw.trim();
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return None;
    }
    if !id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return None;
    }
    Some(id.to_string())
}

/// 表示名から制御文字を除き、長すぎる場合は切り詰める。空なら ID で代用する。
fn normalize_name(raw: &str, fallback_id: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    let base = if trimmed.is_empty() {
        fallback_id
    } else {
        trimmed
    };
    base.chars().take(MAX_NAME_CHARS).collect()
}

pub fn profile_url(id: &str) -> String {
    format!("{PROFILE_URL_PREFIX}{id}")
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 読み込み時にも検証を通し、壊れた行や重複は捨てる。
fn sanitize(list: Vec<FavoriteUser>) -> Vec<FavoriteUser> {
    let mut result: Vec<FavoriteUser> = Vec::new();
    for entry in list {
        let Some(id) = normalize_id(&entry.id) else {
            continue;
        };
        if result.iter().any(|f: &FavoriteUser| f.id == id) {
            continue;
        }
        let name = normalize_name(&entry.name, &id);
        result.push(FavoriteUser {
            id,
            name,
            added_at: entry.added_at,
        });
        if result.len() >= MAX_FAVORITES {
            break;
        }
    }
    result
}

fn store_path<R: Runtime>(app: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(STORE_FILE))
        .map_err(|e| e.to_string())
}

/// 起動時の読み込み。ファイルが無い・壊れている場合は空一覧で始める。
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Vec<FavoriteUser> {
    let Ok(path) = store_path(app) else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    match serde_json::from_str::<Vec<FavoriteUser>>(&raw) {
        Ok(list) => sanitize(list),
        Err(err) => {
            eprintln!("favorites: failed to parse {STORE_FILE}: {err}");
            Vec::new()
        }
    }
}

fn save<R: Runtime>(app: &AppHandle<R>, list: &[FavoriteUser]) -> Result<(), String> {
    let path = store_path(app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// eval へ埋め込む JSON。JS のリテラルとして解釈できない行区切り文字だけ潰す。
fn eval_payload(list: &[FavoriteUser]) -> String {
    serde_json::to_string(list)
        .unwrap_or_else(|_| "[]".to_string())
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// サイドバーへはイベントで、リベシティのページへは eval で配る。
/// ページ側にイベント購読の権限を渡さずに済ませるための非対称な作り。
fn broadcast<R: Runtime>(app: &AppHandle<R>, list: &[FavoriteUser]) {
    let _ = app.emit("favorites-changed", list);

    let script = format!(
        "window.__libeDeskFavorites && window.__libeDeskFavorites.apply({})",
        eval_payload(list)
    );
    for (label, webview) in app.webviews() {
        if label.starts_with("tab-") {
            let _ = webview.eval(script.as_str());
        }
    }
}

fn commit<R: Runtime>(
    app: &AppHandle<R>,
    state: &tauri::State<'_, FavoriteStore>,
) -> Result<Vec<FavoriteUser>, String> {
    let list = state.lock().unwrap().clone();
    save(app, &list)?;
    broadcast(app, &list);
    Ok(list)
}

#[tauri::command]
pub fn list_favorite_users(state: tauri::State<'_, FavoriteStore>) -> Vec<FavoriteUser> {
    state.lock().unwrap().clone()
}

#[tauri::command]
pub fn add_favorite_user<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, FavoriteStore>,
    id: String,
    name: String,
) -> Result<Vec<FavoriteUser>, String> {
    let id = normalize_id(&id).ok_or("invalid user id")?;
    let name = normalize_name(&name, &id);

    {
        let mut guard = state.lock().unwrap();
        if let Some(existing) = guard.iter_mut().find(|f| f.id == id) {
            // 既に登録済みなら、より新しい表示名で上書きするだけに留める。
            existing.name = name;
        } else {
            if guard.len() >= MAX_FAVORITES {
                return Err(format!("お気に入りユーザーは{MAX_FAVORITES}件までです"));
            }
            guard.push(FavoriteUser {
                id,
                name,
                added_at: now_millis(),
            });
        }
    }

    commit(&app, &state)
}

#[tauri::command]
pub fn remove_favorite_user<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, FavoriteStore>,
    id: String,
) -> Result<Vec<FavoriteUser>, String> {
    let id = normalize_id(&id).ok_or("invalid user id")?;
    state.lock().unwrap().retain(|f| f.id != id);
    commit(&app, &state)
}

#[tauri::command]
pub fn rename_favorite_user<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, FavoriteStore>,
    id: String,
    name: String,
) -> Result<Vec<FavoriteUser>, String> {
    let id = normalize_id(&id).ok_or("invalid user id")?;
    {
        let mut guard = state.lock().unwrap();
        let target = guard
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or("favorite user not found")?;
        target.name = normalize_name(&name, &id);
    }
    commit(&app, &state)
}

#[tauri::command]
pub fn move_favorite_user<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, FavoriteStore>,
    id: String,
    to_index: usize,
) -> Result<Vec<FavoriteUser>, String> {
    let id = normalize_id(&id).ok_or("invalid user id")?;
    {
        let mut guard = state.lock().unwrap();
        let from_index = guard
            .iter()
            .position(|f| f.id == id)
            .ok_or("favorite user not found")?;
        if to_index >= guard.len() {
            return Err("index out of range".to_string());
        }
        if from_index == to_index {
            return Ok(guard.clone());
        }
        let entry = guard.remove(from_index);
        guard.insert(to_index, entry);
    }
    commit(&app, &state)
}

#[tauri::command]
pub fn open_favorite_user<R: Runtime>(
    app: AppHandle<R>,
    favorites: tauri::State<'_, FavoriteStore>,
    tabs: tauri::State<'_, crate::tab_manager::TabManager>,
    id: String,
) -> Result<crate::tab_manager::TabInfo, String> {
    let id = normalize_id(&id).ok_or("invalid user id")?;
    let name = favorites
        .lock()
        .unwrap()
        .iter()
        .find(|f| f.id == id)
        .map(|f| f.name.clone())
        .ok_or("favorite user not found")?;

    let url = profile_url(&id);
    if let Some(tab) = crate::tab_manager::reuse_url(&app, &tabs, &url)? {
        return Ok(tab);
    }
    crate::tab_manager::open_url_internal_with_title(&app, &tabs, &url, Some(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn favorite(id: &str, name: &str) -> FavoriteUser {
        FavoriteUser {
            id: id.to_string(),
            name: name.to_string(),
            added_at: 0,
        }
    }

    #[test]
    fn ids_accept_only_url_safe_shapes() {
        assert_eq!(normalize_id("abc123"), Some("abc123".to_string()));
        assert_eq!(normalize_id("  a-b_c  "), Some("a-b_c".to_string()));

        for invalid in [
            "",
            "   ",
            "a/b",
            "../etc",
            "a?b",
            "a b",
            "https://libecity.com/",
            "ユーザー",
        ] {
            assert_eq!(normalize_id(invalid), None, "{invalid}");
        }

        assert_eq!(normalize_id(&"a".repeat(MAX_ID_LEN + 1)), None);
        assert!(normalize_id(&"a".repeat(MAX_ID_LEN)).is_some());
    }

    #[test]
    fn names_drop_control_chars_and_truncate() {
        assert_eq!(normalize_name(" くま \n", "u1"), "くま");
        assert_eq!(normalize_name("a\u{0}b", "u1"), "ab");
        assert_eq!(normalize_name("", "u1"), "u1");
        assert_eq!(normalize_name("   ", "u1"), "u1");
        assert_eq!(
            normalize_name(&"あ".repeat(MAX_NAME_CHARS + 10), "u1")
                .chars()
                .count(),
            MAX_NAME_CHARS
        );
    }

    #[test]
    fn sanitize_drops_invalid_and_duplicate_entries() {
        let list = sanitize(vec![
            favorite("u1", "One"),
            favorite("u1", "Duplicate"),
            favorite("bad id", "Invalid"),
            favorite("u2", ""),
        ]);

        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "One");
        assert_eq!(list[1].id, "u2");
        assert_eq!(list[1].name, "u2");
    }

    #[test]
    fn sanitize_enforces_the_upper_bound() {
        let list: Vec<FavoriteUser> = (0..MAX_FAVORITES + 20)
            .map(|i| favorite(&format!("u{i}"), "name"))
            .collect();
        assert_eq!(sanitize(list).len(), MAX_FAVORITES);
    }

    #[test]
    fn profile_urls_stay_on_the_libecity_host() {
        assert_eq!(
            profile_url("abc123"),
            "https://libecity.com/user_profile/abc123"
        );
    }

    #[test]
    fn eval_payload_escapes_js_line_separators() {
        let payload = eval_payload(&[favorite("u1", "a\u{2028}b")]);
        assert!(!payload.contains('\u{2028}'));
        assert!(payload.contains("\\u2028"));
    }
}

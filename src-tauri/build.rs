fn main() {
    // アプリのコマンドを ACL の管理下に置く。これを定義すると自作コマンドも
    // すべて capability での明示的な許可が必要になるため、リベシティのページ
    // （リモート origin）へ渡すコマンドをお気に入りの3つだけに絞れる。
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "list_services",
            "open_service",
            "list_tabs",
            "switch_tab",
            "close_tab",
            "move_tab",
            "go_back",
            "go_forward",
            "reload_tab",
            "get_current_page",
            "apply_chrome_layout",
            "list_favorite_users",
            "add_favorite_user",
            "remove_favorite_user",
            "rename_favorite_user",
            "move_favorite_user",
            "open_favorite_user",
        ]),
    ))
    .expect("failed to run tauri-build");
}

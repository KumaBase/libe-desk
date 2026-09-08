import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ServiceInfo {
  id: string;
  name: string;
  url: string;
  category: string;
  pinned: boolean;
  external: boolean;
}

export interface TabInfo {
  id: string;
  title: string;
  url: string;
  active: boolean;
}

/** リベシティのプロフィールページへのローカルブックマーク */
export interface FavoriteUser {
  id: string;
  name: string;
  addedAt: number;
}

/**
 * UI(SolidJS)と実際のタブ・ページ操作を分離するための共通操作口。
 * ここに定義された関数だけを介してタブ・ページを操作することで、
 * 将来 MCP サーバーから同じ操作を呼び出せるようにする。
 */
export const controller = {
  listServices: () => invoke<ServiceInfo[]>("list_services"),
  openService: (serviceId: string) =>
    invoke<TabInfo | null>("open_service", { serviceId }),
  listTabs: () => invoke<TabInfo[]>("list_tabs"),
  moveTab: (tabId: string, beforeTabId: string | null) =>
    invoke<void>("move_tab", { tabId, beforeTabId }),
  switchTab: (tabId: string) => invoke<void>("switch_tab", { tabId }),
  closeTab: (tabId: string) => invoke<void>("close_tab", { tabId }),
  goBack: (tabId: string) => invoke<void>("go_back", { tabId }),
  goForward: (tabId: string) => invoke<void>("go_forward", { tabId }),
  reloadTab: (tabId: string) => invoke<void>("reload_tab", { tabId }),
  getCurrentPage: (tabId: string) =>
    invoke<TabInfo>("get_current_page", { tabId }),
  applyChromeLayout: () => invoke<void>("apply_chrome_layout"),
  onTabsChanged: (cb: (tabs: TabInfo[]) => void): Promise<UnlistenFn> =>
    listen<TabInfo[]>("tabs-changed", (event) => cb(event.payload)),

  listFavoriteUsers: () => invoke<FavoriteUser[]>("list_favorite_users"),
  addFavoriteUser: (id: string, name: string) =>
    invoke<FavoriteUser[]>("add_favorite_user", { id, name }),
  removeFavoriteUser: (id: string) =>
    invoke<FavoriteUser[]>("remove_favorite_user", { id }),
  renameFavoriteUser: (id: string, name: string) =>
    invoke<FavoriteUser[]>("rename_favorite_user", { id, name }),
  moveFavoriteUser: (id: string, toIndex: number) =>
    invoke<FavoriteUser[]>("move_favorite_user", { id, toIndex }),
  openFavoriteUser: (id: string) =>
    invoke<TabInfo>("open_favorite_user", { id }),
  onFavoritesChanged: (
    cb: (favorites: FavoriteUser[]) => void,
  ): Promise<UnlistenFn> =>
    listen<FavoriteUser[]>("favorites-changed", (event) => cb(event.payload)),
};

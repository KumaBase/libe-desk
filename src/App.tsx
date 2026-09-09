import { onMount } from "solid-js";
import Sidebar from "./components/Sidebar";
import TabBar from "./components/TabBar";
import { controller } from "./lib/controller";
import { initFavoritesStore } from "./store/favorites";
import { initTabsStore } from "./store/tabs";
import "./App.css";

/**
 * 専用のウィンドウ移動領域の下に独立したタブ帯。
 * コンテンツ Webview はタイトルバー下 × サイドバー右。
 */
function App() {
  onMount(() => {
    initTabsStore();
    initFavoritesStore();
    void (async () => {
      await controller.applyChromeLayout();
      const tabs = await controller.listTabs();
      if (tabs.length === 0) {
        await controller.openService("libecity");
      }
      await controller.applyChromeLayout();
    })();
  });

  return (
    <div class="app-shell">
      <div class="window-dragbar" data-tauri-drag-region title="ドラッグで移動・ダブルクリックで拡大／元に戻す">
        <span>Libe Desk</span>
      </div>
      <header class="titlebar">
        <div class="workspace-label">開いているページ</div>
        <TabBar />
      </header>
      <div class="app-body">
        <Sidebar />
        <div class="content-hole" aria-hidden="true" />
      </div>
    </div>
  );
}

export default App;

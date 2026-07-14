import { onMount } from "solid-js";
import Sidebar from "./components/Sidebar";
import TabBar from "./components/TabBar";
import { controller } from "./lib/controller";
import { initTabsStore } from "./store/tabs";
import "./App.css";

/**
 * Overlay タイトルバー内（信号ボタン右）にタブ。
 * コンテンツ Webview はタイトルバー下 × サイドバー右。
 */
function App() {
  onMount(() => {
    initTabsStore();
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
      <header class="titlebar" data-tauri-drag-region>
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

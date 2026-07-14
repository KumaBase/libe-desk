import { For } from "solid-js";
import { controller } from "../lib/controller";
import { tabsList } from "../store/tabs";

function tabLabel(title: string, url: string): string {
  const t = title.trim();
  if (!t || t.startsWith("http://") || t.startsWith("https://")) {
    try {
      return new URL(url).hostname.replace(/^www\./, "");
    } catch {
      return "ページ";
    }
  }
  return t;
}

export default function TabBar() {
  return (
    <div class="tab-bar">
      <div class="tab-list">
        <For each={tabsList()}>
          {(tab) => (
            <div
              class="tab"
              classList={{ "tab-active": tab.active }}
              onClick={() => controller.switchTab(tab.id)}
              title={tabLabel(tab.title, tab.url)}
            >
              <span class="tab-title">{tabLabel(tab.title, tab.url)}</span>
              <button
                class="tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  controller.closeTab(tab.id);
                }}
                aria-label="タブを閉じる"
              >
                ×
              </button>
            </div>
          )}
        </For>
      </div>
      <button
        class="tab-new"
        onClick={() => controller.openService("libecity")}
        aria-label="新しいタブ"
        title="新しいタブ"
      >
        +
      </button>
    </div>
  );
}

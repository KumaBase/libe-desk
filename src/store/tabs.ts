import { createStore } from "solid-js/store";
import { controller, type TabInfo } from "../lib/controller";

const [state, setState] = createStore<{ tabs: TabInfo[] }>({ tabs: [] });

let initialized = false;

export function initTabsStore() {
  if (initialized) return;
  initialized = true;

  controller.listTabs().then((tabs) => setState("tabs", tabs));
  controller.onTabsChanged((tabs) => setState("tabs", tabs));
}

export function tabsList(): TabInfo[] {
  return state.tabs;
}

export function activeTab(): TabInfo | undefined {
  return state.tabs.find((t) => t.active);
}

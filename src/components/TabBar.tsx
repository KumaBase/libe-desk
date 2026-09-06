import { For, createSignal, onCleanup } from "solid-js";
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
  let list!: HTMLDivElement;
  let pointer: { id: number; tabId: string; startX: number; x: number; y: number; button: HTMLButtonElement } | undefined;
  let frame = 0;
  let suppressClick = false;
  const [dragging, setDragging] = createSignal<string | null>(null);
  const [destination, setDestination] = createSignal<string | null | undefined>(undefined);
  const [error, setError] = createSignal("");

  function updateDestination() {
    if (!pointer) return;
    const bounds = list.getBoundingClientRect();
    if (pointer.y < bounds.top - 16 || pointer.y > bounds.bottom + 16 ||
        pointer.x < bounds.left - 24 || pointer.x > bounds.right + 24) {
      setDestination(undefined);
      return;
    }
    const candidates = [...list.querySelectorAll<HTMLElement>("[data-tab-id]")]
      .filter((element) => element.dataset.tabId !== pointer!.tabId);
    const before = candidates.find((element) => {
      const rect = element.getBoundingClientRect();
      return pointer!.x < rect.left + rect.width / 2;
    });
    setDestination(before?.dataset.tabId ?? null);
  }

  function scrollWhileDragging() {
    if (!pointer || !dragging()) return;
    const bounds = list.getBoundingClientRect();
    if (destination() !== undefined) {
      const direction = pointer.x < bounds.left + 32 ? -1 : pointer.x > bounds.right - 32 ? 1 : 0;
      list.scrollLeft += direction * 9;
      updateDestination();
    }
    frame = requestAnimationFrame(scrollWhileDragging);
  }

  function resetDrag() {
    cancelAnimationFrame(frame);
    const previous = pointer;
    pointer = undefined;
    setDragging(null);
    setDestination(undefined);
    if (previous?.button.hasPointerCapture(previous.id)) {
      previous.button.releasePointerCapture(previous.id);
    }
  }

  function cancelDrag() {
    if (dragging()) suppressClick = true;
    resetDrag();
  }

  const escape = (event: KeyboardEvent) => {
    if (event.key === "Escape" && pointer) cancelDrag();
  };
  window.addEventListener("keydown", escape);
  window.addEventListener("blur", cancelDrag);
  onCleanup(() => {
    resetDrag();
    window.removeEventListener("keydown", escape);
    window.removeEventListener("blur", cancelDrag);
  });

  function startDrag(event: PointerEvent & { currentTarget: HTMLButtonElement }, tabId: string) {
    if (event.button !== 0 || !event.isPrimary) return;
    suppressClick = false;
    pointer = { id: event.pointerId, tabId, startX: event.clientX, x: event.clientX, y: event.clientY, button: event.currentTarget };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function moveDrag(event: PointerEvent) {
    if (!pointer || pointer.id !== event.pointerId) return;
    pointer.x = event.clientX;
    pointer.y = event.clientY;
    if (!dragging() && Math.abs(pointer.x - pointer.startX) >= 6) {
      setDragging(pointer.tabId);
      setError("");
      frame = requestAnimationFrame(scrollWhileDragging);
    }
    if (dragging()) {
      event.preventDefault();
      updateDestination();
    }
  }

  function finishDrag(event: PointerEvent) {
    if (!pointer || pointer.id !== event.pointerId) return;
    const tabId = dragging();
    updateDestination();
    const before = destination();
    suppressClick = !!tabId;
    resetDrag();
    if (tabId && before !== undefined) {
      void controller.moveTab(tabId, before).catch(() => setError("タブの並べ替えに失敗しました。もう一度お試しください。"));
    }
  }
  return (
    <div class="tab-bar">
      <div ref={list} class="tab-list" role="tablist" aria-label="開いているページ">
        <For each={tabsList()}>
          {(tab) => (
            <div
              class="tab"
              data-tab-id={tab.id}
              classList={{
                "tab-active": tab.active,
                "tab-dragging": dragging() === tab.id,
                "tab-drop-before": dragging() !== null && destination() === tab.id,
                "tab-drop-after": dragging() !== null && destination() === null && tabsList()[tabsList().length - 1]?.id === tab.id,
              }}
              role="presentation"
            >
              <button
                class="tab-select"
                role="tab"
                aria-selected={tab.active}
                title={`${tabLabel(tab.title, tab.url)}（ドラッグで並べ替え）`}
                onPointerDown={(event) => startDrag(event, tab.id)}
                onPointerMove={moveDrag}
                onPointerUp={finishDrag}
                onPointerCancel={cancelDrag}
                onLostPointerCapture={cancelDrag}
                onDragStart={(event) => event.preventDefault()}
                onClick={(event) => { if (!suppressClick || event.detail === 0) void controller.switchTab(tab.id); }}
              >
                <span class="tab-title">{tabLabel(tab.title, tab.url)}</span>
              </button>
              <button
                class="tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  controller.closeTab(tab.id);
                }}
                aria-label={`${tabLabel(tab.title, tab.url)}のタブを閉じる`}
                title="タブを閉じる（⌘W / Ctrl+W）"
              >
                ×
              </button>
            </div>
          )}
        </For>
      </div>
      <span class="tab-reorder-error" role="alert">{error()}</span>
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

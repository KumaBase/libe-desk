import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { controller, type ServiceInfo } from "../lib/controller";
import { activeTab } from "../store/tabs";

const CATEGORY_ORDER = ["本体", "学ぶ", "交流する", "仕事・副業・売買"] as const;

/** 常に「よく使う」に出す（ピン外し不可） */
const FIXED_PINNED_ID = "libecity";
const STORAGE_KEY = "libe-desk.pinned-service-ids";

function loadExtraPinned(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((id): id is string => typeof id === "string")
      .filter((id) => id !== FIXED_PINNED_ID);
  } catch {
    return [];
  }
}

function saveExtraPinned(ids: string[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(ids));
}

function PinIcon(props: { filled: boolean }) {
  return (
    <svg
      class="sidebar-pin-icon"
      viewBox="0 0 16 16"
      width="14"
      height="14"
      aria-hidden="true"
    >
      <path
        fill={props.filled ? "currentColor" : "none"}
        stroke="currentColor"
        stroke-width="1.4"
        d="M8.5 1.5 10 5.2l3.8.4-2.9 2.6.9 3.7L8 9.8l-3.8 2.1.9-3.7L2.2 5.6l3.8-.4L8.5 1.5z"
        stroke-linejoin="round"
      />
    </svg>
  );
}

function ServiceRow(props: {
  service: ServiceInfo;
  pinned: boolean;
  fixed?: boolean;
  onOpen: () => void;
  onTogglePin: (e: MouseEvent) => void;
}) {
  return (
    <div class="sidebar-item-row">
      <button class="sidebar-item" onClick={props.onOpen}
        title={props.service.external ? `${props.service.name}（確認後にブラウザで開く）` : props.service.name}>
        {props.service.name}
        <Show when={props.service.external}><span aria-hidden="true"> ↗</span></Show>
      </button>
      <button
        class="sidebar-pin"
        classList={{
          "sidebar-pin-active": props.pinned,
          "sidebar-pin-fixed": props.fixed,
        }}
        disabled={props.fixed}
        onClick={props.onTogglePin}
        title={
          props.fixed
            ? "リベシティは常に表示"
            : props.pinned
              ? "ピン留めを外す"
              : "よく使うにピン留め"
        }
        aria-label={
          props.fixed
            ? "リベシティは常に表示"
            : props.pinned
              ? "ピン留めを外す"
              : "よく使うにピン留め"
        }
        aria-pressed={props.pinned}
      >
        <PinIcon filled={props.pinned} />
      </button>
    </div>
  );
}

export default function Sidebar() {
  const [services] = createResource(() => controller.listServices());
  const [allOpen, setAllOpen] = createSignal(false);
  const [extraPinned, setExtraPinned] = createSignal(loadExtraPinned());
  const tabId = () => activeTab()?.id;

  const isPinned = (id: string) =>
    id === FIXED_PINNED_ID || extraPinned().includes(id);

  const pinned = createMemo(() => {
    const byId = new Map((services() ?? []).map((s) => [s.id, s]));
    const ordered = [
      FIXED_PINNED_ID,
      ...extraPinned().filter((id) => id !== FIXED_PINNED_ID),
    ];
    return ordered
      .map((id) => byId.get(id))
      .filter((s): s is ServiceInfo => s != null);
  });

  const grouped = createMemo(() => {
    const all = services() ?? [];
    return CATEGORY_ORDER.map((category) => ({
      category,
      items: all.filter((s) => s.category === category),
    })).filter((g) => g.items.length > 0);
  });

  function openService(service: ServiceInfo) {
    controller.openService(service.id);
  }

  function togglePin(serviceId: string, e: MouseEvent) {
    e.stopPropagation();
    if (serviceId === FIXED_PINNED_ID) return;
    setExtraPinned((prev) => {
      const next = prev.includes(serviceId)
        ? prev.filter((id) => id !== serviceId)
        : [...prev, serviceId];
      saveExtraPinned(next);
      return next;
    });
  }

  return (
    <aside class="sidebar">
      <div class="sidebar-header">Libe Desk</div>

      <div class="sidebar-nav-controls">
        <button
          class="sidebar-nav-btn"
          disabled={!tabId()}
          onClick={() => {
            const id = tabId();
            if (id) controller.goBack(id);
          }}
          aria-label="戻る"
          title="戻る"
        >
          ←
        </button>
        <button
          class="sidebar-nav-btn"
          disabled={!tabId()}
          onClick={() => {
            const id = tabId();
            if (id) controller.goForward(id);
          }}
          aria-label="進む"
          title="進む"
        >
          →
        </button>
        <button
          class="sidebar-nav-btn"
          disabled={!tabId()}
          onClick={() => {
            const id = tabId();
            if (id) controller.reloadTab(id);
          }}
          aria-label="再読み込み"
          title="再読み込み"
        >
          ⟳
        </button>
      </div>

      <div class="sidebar-scroll">
        <div class="sidebar-section-label">よく使う</div>
        <nav class="sidebar-nav">
          <For each={pinned()}>
            {(service) => (
              <ServiceRow
                service={service}
                pinned
                fixed={service.id === FIXED_PINNED_ID}
                onOpen={() => openService(service)}
                onTogglePin={(e) => togglePin(service.id, e)}
              />
            )}
          </For>
        </nav>

        <button
          class="sidebar-all-toggle"
          aria-expanded={allOpen()}
          onClick={() => setAllOpen((v) => !v)}
        >
          <span>すべてのサービス</span>
          <span class="sidebar-all-chevron" classList={{ open: allOpen() }}>
            ▾
          </span>
        </button>

        <Show when={allOpen()}>
          <div class="sidebar-all-services">
            <For each={grouped()}>
              {(group) => (
                <div class="sidebar-category">
                  <div class="sidebar-category-label">{group.category}</div>
                  <nav class="sidebar-nav">
                    <For each={group.items}>
                      {(service) => (
                        <ServiceRow
                          service={service}
                          pinned={isPinned(service.id)}
                          fixed={service.id === FIXED_PINNED_ID}
                          onOpen={() => openService(service)}
                          onTogglePin={(e) => togglePin(service.id, e)}
                        />
                      )}
                    </For>
                  </nav>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="sidebar-footer">
        <p class="sidebar-hint">
          ピンで「よく使う」に追加。リベシティは常時表示
        </p>
        <span class="unofficial-badge">非公式アプリ</span>
      </div>
    </aside>
  );
}

import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import {
  controller,
  type FavoriteUser,
  type ServiceInfo,
} from "../lib/controller";
import { applyFavorites, favoritesList } from "../store/favorites";
import { activeTab } from "../store/tabs";

const CATEGORY_ORDER = ["本体", "学ぶ", "交流する", "仕事・副業・売買"] as const;

/** 常に「よく使う」に出す（ピン外し不可） */
const FIXED_PINNED_ID = "libecity";
const STORAGE_KEY = "libe-desk.pinned-service-ids";
const TAB_STORAGE_KEY = "libe-desk.sidebar-tab";

/** サイドバー本体の表示切り替え。サービスとユーザーは別物なので混ぜない。 */
type SidebarTab = "links" | "users";

function loadSidebarTab(): SidebarTab {
  return localStorage.getItem(TAB_STORAGE_KEY) === "users" ? "users" : "links";
}

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
        title={props.service.external ? `${props.service.name}（新しいタブで開く）` : props.service.name}>
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

/**
 * お気に入りユーザーの1行。名前はその場で編集でき、行はドラッグで並べ替える。
 * 登録・解除自体はリベシティのページに出る★から行う。
 */
function FavoriteUserRow(props: {
  favorite: FavoriteUser;
  dragging: boolean;
  onDragStart: (e: DragEvent) => void;
  onDragOver: (e: DragEvent) => void;
  onDrop: (e: DragEvent) => void;
  onDragEnd: () => void;
}) {
  const [editing, setEditing] = createSignal(false);
  let cancelled = false;

  function commit(value: string) {
    setEditing(false);
    const next = value.trim();
    if (!next || next === props.favorite.name) return;
    void controller
      .renameFavoriteUser(props.favorite.id, next)
      .then(applyFavorites);
  }

  return (
    <div
      class="sidebar-item-row sidebar-fav-row"
      classList={{ "sidebar-fav-dragging": props.dragging }}
      draggable={!editing()}
      onDragStart={props.onDragStart}
      onDragOver={props.onDragOver}
      onDrop={props.onDrop}
      onDragEnd={props.onDragEnd}
    >
      <Show
        when={editing()}
        fallback={
          <button
            class="sidebar-item"
            onClick={() => controller.openFavoriteUser(props.favorite.id)}
            title={props.favorite.name}
          >
            {props.favorite.name}
          </button>
        }
      >
        <input
          class="sidebar-fav-input"
          value={props.favorite.name}
          ref={(el) => queueMicrotask(() => el.select())}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.currentTarget.blur();
            } else if (e.key === "Escape") {
              cancelled = true;
              e.currentTarget.blur();
            }
          }}
          onBlur={(e) => {
            if (cancelled) {
              cancelled = false;
              setEditing(false);
              return;
            }
            commit(e.currentTarget.value);
          }}
        />
      </Show>
      <button
        class="sidebar-fav-action"
        onClick={() => setEditing(true)}
        title="名前を変更"
        aria-label="名前を変更"
      >
        ✎
      </button>
      <button
        class="sidebar-fav-action"
        onClick={() =>
          void controller
            .removeFavoriteUser(props.favorite.id)
            .then(applyFavorites)
        }
        title="お気に入りから外す"
        aria-label="お気に入りから外す"
      >
        ×
      </button>
    </div>
  );
}

export default function Sidebar() {
  const [services] = createResource(() => controller.listServices());
  const [allOpen, setAllOpen] = createSignal(false);
  const [extraPinned, setExtraPinned] = createSignal(loadExtraPinned());
  const [draggingFavId, setDraggingFavId] = createSignal<string | null>(null);
  const [sidebarTab, setSidebarTab] = createSignal<SidebarTab>(loadSidebarTab());
  const tabId = () => activeTab()?.id;

  let linksTabRef: HTMLButtonElement | undefined;
  let usersTabRef: HTMLButtonElement | undefined;

  function selectSidebarTab(next: SidebarTab) {
    setSidebarTab(next);
    localStorage.setItem(TAB_STORAGE_KEY, next);
  }

  // tablist の作法として左右キーでも切り替えられるようにする。
  function onSidebarTabKeyDown(e: KeyboardEvent) {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const next = sidebarTab() === "links" ? "users" : "links";
    selectSidebarTab(next);
    (next === "links" ? linksTabRef : usersTabRef)?.focus();
  }

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
    controller.openService(service.id, true);
  }

  function onFavDrop(targetId: string) {
    const sourceId = draggingFavId();
    setDraggingFavId(null);
    if (!sourceId || sourceId === targetId) return;
    const toIndex = favoritesList().findIndex((f) => f.id === targetId);
    if (toIndex < 0) return;
    void controller.moveFavoriteUser(sourceId, toIndex).then(applyFavorites);
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

      <div class="sidebar-tabs" role="tablist" aria-label="サイドバーの表示切り替え">
        <button
          ref={linksTabRef}
          type="button"
          role="tab"
          id="sidebar-tab-links"
          class="sidebar-tab"
          classList={{ "sidebar-tab-active": sidebarTab() === "links" }}
          aria-selected={sidebarTab() === "links"}
          aria-controls="sidebar-panel-links"
          tabindex={sidebarTab() === "links" ? 0 : -1}
          title="よく使うサービス"
          onClick={() => selectSidebarTab("links")}
          onKeyDown={onSidebarTabKeyDown}
        >
          よく使う
        </button>
        <button
          ref={usersTabRef}
          type="button"
          role="tab"
          id="sidebar-tab-users"
          class="sidebar-tab"
          classList={{ "sidebar-tab-active": sidebarTab() === "users" }}
          aria-selected={sidebarTab() === "users"}
          aria-controls="sidebar-panel-users"
          tabindex={sidebarTab() === "users" ? 0 : -1}
          title="お気に入りユーザー"
          onClick={() => selectSidebarTab("users")}
          onKeyDown={onSidebarTabKeyDown}
        >
          ユーザー
          <Show when={favoritesList().length > 0}>
            <span class="sidebar-tab-count">{favoritesList().length}</span>
          </Show>
        </button>
      </div>

      <div class="sidebar-scroll">
        <Show when={sidebarTab() === "links"}>
          <div
            id="sidebar-panel-links"
            role="tabpanel"
            aria-labelledby="sidebar-tab-links"
          >
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
        </Show>

        <Show when={sidebarTab() === "users"}>
          <div
            id="sidebar-panel-users"
            role="tabpanel"
            aria-labelledby="sidebar-tab-users"
          >
            <Show
              when={favoritesList().length > 0}
              fallback={
                <p class="sidebar-empty">
                  リベシティのプロフィールにある★から追加できます
                </p>
              }
            >
              <nav class="sidebar-nav">
                <For each={favoritesList()}>
                  {(favorite) => (
                    <FavoriteUserRow
                      favorite={favorite}
                      dragging={draggingFavId() === favorite.id}
                      onDragStart={(e) => {
                        setDraggingFavId(favorite.id);
                        if (e.dataTransfer) {
                          e.dataTransfer.effectAllowed = "move";
                          // Firefox 等でドラッグを有効化するため setData が必要
                          e.dataTransfer.setData("text/plain", favorite.id);
                        }
                      }}
                      onDragOver={(e) => {
                        if (!draggingFavId()) return;
                        e.preventDefault();
                        if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
                      }}
                      onDrop={(e) => {
                        e.preventDefault();
                        onFavDrop(favorite.id);
                      }}
                      onDragEnd={() => setDraggingFavId(null)}
                    />
                  )}
                </For>
              </nav>
            </Show>
          </div>
        </Show>
      </div>

      <div class="sidebar-footer">
        <p class="sidebar-hint">
          {sidebarTab() === "links"
            ? "ピンで「よく使う」に追加。リベシティは常時表示"
            : "★で追加。ドラッグで並べ替え、✎で名前を変更"}
        </p>
        <span class="unofficial-badge">非公式アプリ</span>
      </div>
    </aside>
  );
}

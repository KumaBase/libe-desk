import { createStore } from "solid-js/store";
import { controller, type FavoriteUser } from "../lib/controller";

const [state, setState] = createStore<{ favorites: FavoriteUser[] }>({
  favorites: [],
});

let initialized = false;

export function initFavoritesStore() {
  if (initialized) return;
  initialized = true;

  controller
    .listFavoriteUsers()
    .then((favorites) => setState("favorites", favorites));
  controller.onFavoritesChanged((favorites) =>
    setState("favorites", favorites),
  );
}

export function favoritesList(): FavoriteUser[] {
  return state.favorites;
}

/** 各操作は更新後の一覧を返すので、イベントを待たずに即反映する。 */
export function applyFavorites(favorites: FavoriteUser[]) {
  setState("favorites", favorites);
}

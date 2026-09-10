// リベシティのページへ注入し、プロフィールへのリンクとプロフィール画面に
// お気に入り登録用の★ボタンを描画する。
//
// アプリ側とは window.__TAURI_INTERNALS__.invoke でやり取りする。呼べるのは
// capabilities/libecity-page.json で許可した3つのコマンドだけ。
//
// リベシティは非公式の対象なので DOM 構造は変わりうる。目印は
// a[href^="/user_profile/"] を主軸にし、失敗しても本来のページ表示を
// 壊さないよう全体を try/catch で囲う。
;(function () {
  "use strict";

  if (window.location.origin !== "https://libecity.com") return;
  if (window.__libeDeskFavorites) return;

  var internals = window.__TAURI_INTERNALS__;
  if (!internals || typeof internals.invoke !== "function") return;

  var PROFILE_PATH = /^\/user_profile\/([^/?#]+)/;
  var ID_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;
  var MAX_NAME_CHARS = 60;
  var BUTTON_CLASS = "libedesk-fav-btn";
  var STYLE_ID = "libedesk-fav-style";
  var STAR_PATH = "M8.5 1.5 10 5.2l3.8.4-2.9 2.6.9 3.7L8 9.8l-3.8 2.1.9-3.7L2.2 5.6l3.8-.4L8.5 1.5z";

  var favoriteIds = {};
  var pending = {};
  var profileButton = null;
  var scheduled = 0;
  var dirtyRoots = new Set();
  var fullRefresh = true;

  function invoke(cmd, args) {
    return internals.invoke(cmd, args || {});
  }

  function idFromPath(pathname) {
    var matched = PROFILE_PATH.exec(pathname || "");
    if (!matched) return null;
    var id;
    try {
      id = decodeURIComponent(matched[1]);
    } catch (err) {
      return null;
    }
    return ID_PATTERN.test(id) ? id : null;
  }

  function idFromHref(href) {
    try {
      var url = new URL(href, window.location.origin);
      return url.origin === window.location.origin ? idFromPath(url.pathname) : null;
    } catch (err) {
      return null;
    }
  }

  function truncate(value) {
    return value.length > MAX_NAME_CHARS ? value.slice(0, MAX_NAME_CHARS) : value;
  }

  // 一覧側のリンクはアイコンだけのこともあるので、テキスト → alt → ID の順で拾う。
  function anchorName(anchor, id) {
    var text = (anchor.textContent || "").trim();
    if (text) return truncate(text);
    var image = anchor.querySelector("img[alt]");
    if (image) {
      var alt = (image.getAttribute("alt") || "").trim();
      if (alt) return truncate(alt);
    }
    return id;
  }

  // プロフィール画面の名前。.username は .userprof_wrap の子ではなく兄弟で、
  // 中身は素のテキスト（リンクではない）。
  function profileNode() {
    return document.querySelector(".user_profdata .username, .username");
  }

  function profileName(id) {
    var node = profileNode();
    var text = node ? (node.textContent || "").trim() : "";
    return text ? truncate(text) : id;
  }

  function paint(button) {
    var on = Object.prototype.hasOwnProperty.call(favoriteIds, button.dataset.libedeskId);
    var label = on ? "お気に入りユーザーから外す" : "お気に入りユーザーに追加";
    button.classList.toggle("is-on", on);
    button.setAttribute("aria-pressed", on ? "true" : "false");
    button.setAttribute("title", label);
    button.setAttribute("aria-label", label);
    var path = button.firstChild && button.firstChild.firstChild;
    if (path) path.setAttribute("fill", on ? "currentColor" : "none");
  }

  function repaint() {
    var buttons = document.querySelectorAll("." + BUTTON_CLASS);
    for (var i = 0; i < buttons.length; i++) paint(buttons[i]);
  }

  function toggle(id, name) {
    if (pending[id]) return;
    pending[id] = true;

    var remove = Object.prototype.hasOwnProperty.call(favoriteIds, id);
    var request = remove
      ? invoke("remove_favorite_user", { id: id })
      : invoke("add_favorite_user", { id: id, name: name });

    request
      .then(applyList)
      .catch(function (err) {
        console.warn("[Libe Desk] お気に入りの更新に失敗しました", err);
      })
      .then(function () {
        delete pending[id];
      });
  }

  function createButton(id, resolveName) {
    var button = document.createElement("button");
    button.type = "button";
    button.className = BUTTON_CLASS;
    button.dataset.libedeskId = id;
    button.innerHTML =
      '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">' +
      '<path fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" d="' +
      STAR_PATH +
      '"></path></svg>';
    button.addEventListener("click", function (event) {
      event.preventDefault();
      event.stopPropagation();
      toggle(id, resolveName());
    });
    paint(button);
    return button;
  }

  // 同じユーザーへのリンクは1枚のカードに複数ある（アイコンと名前など）。
  // 近い先祖に★が既にあるなら重ねて出さない。
  function hasNearbyButton(node, id) {
    var current = node;
    for (var level = 0; current && level < 4; level++) {
      if (
        current.querySelector &&
        current.querySelector(
          "." + BUTTON_CLASS + '[data-libedesk-id="' + id + '"]'
        )
      ) {
        return true;
      }
      current = current.parentElement;
    }
    return false;
  }

  function decorateAnchors(root) {
    var selector = 'a[href^="/user_profile/"], a[data-libedesk-fav]';
    var anchors = Array.from(root.querySelectorAll(selector));
    if (root.matches && root.matches(selector)) anchors.unshift(root);
    for (var i = 0; i < anchors.length; i++) {
      var anchor = anchors[i];
      var id = idFromHref(anchor.getAttribute("href"));
      var existing = anchor.querySelector("." + BUTTON_CLASS);
      if (existing && existing.dataset.libedeskId === id) continue;
      // 仮想リストで同じリンク要素が別ユーザーに再利用される場合。
      if (existing) existing.remove();
      anchor.removeAttribute("data-libedesk-fav");

      // アイコンだけのリンクには付けない。名前のリンク側にまとめる。
      if (!(anchor.textContent || "").trim()) continue;

      if (!id) {
        anchor.setAttribute("data-libedesk-fav", "");
        continue;
      }
      if (hasNearbyButton(anchor, id)) continue;

      anchor.setAttribute("data-libedesk-fav", "");
      // 名前の直後に流し込みたいので、兄弟ではなくリンクの末尾へ入れる。
      // 兄弟にするとカードの flex 折り返しで次の行へ落ちてしまう。
      anchor.appendChild(
        createButton(
          id,
          (function (node, userId) {
            return function () {
              return anchorName(node, userId);
            };
          })(anchor, id)
        )
      );
    }
  }

  // プロフィール画面本体。SPA でユーザーが切り替わったら作り直す。
  function decorateProfile() {
    var id = idFromPath(window.location.pathname);
    if (!id) {
      if (profileButton) profileButton.remove();
      profileButton = null;
      return;
    }

    if (profileButton && profileButton.dataset.libedeskId !== id) {
      profileButton.remove();
      profileButton = null;
    }
    if (profileButton && profileButton.isConnected) return;

    // 名前の行へ入れる。.userprof_wrap は「リベシティ公式」などのラベル行なので、
    // ここへ入れるとラベルの並びを崩してしまう。
    var host = profileNode();
    if (!host || hasNearbyButton(host, id)) return;

    profileButton = createButton(id, function () {
      return profileName(id);
    });
    profileButton.classList.add("is-profile");
    host.appendChild(profileButton);
  }

  function ensureStyle() {
    if (!document.head || document.getElementById(STYLE_ID)) return;
    var style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = [
      "." + BUTTON_CLASS + "{",
      "display:inline-flex;align-items:center;justify-content:center;",
      "width:18px;height:18px;margin:0 0 0 4px;padding:0;",
      "vertical-align:middle;text-decoration:none;",
      "border:0;border-radius:6px;background:transparent;cursor:pointer;",
      "color:#9aa4b2;line-height:0;flex:none;}",
      "." + BUTTON_CLASS + ":hover{background:rgba(0,0,0,.06);color:#f0a80a;}",
      "." + BUTTON_CLASS + ".is-on{color:#f0a80a;}",
      "." + BUTTON_CLASS + ".is-profile{width:26px;height:26px;margin-left:6px;}",
    ].join("");
    document.head.appendChild(style);
  }

  function queueRoot(node) {
    var element = node.nodeType === 1 ? node : node.parentElement;
    if (!element || !element.isConnected) return;
    if (element.closest("." + BUTTON_CLASS)) return;
    // テキスト追加も、リンク自身を調べれば済む。
    dirtyRoots.add(element.closest("a") || element);
  }

  var observer = new MutationObserver(function (records) {
    if (document.hidden) {
      fullRefresh = true;
      dirtyRoots.clear();
      return;
    }
    records.forEach(function (record) {
      if (record.type === "attributes" || record.type === "characterData") {
        queueRoot(record.target);
      } else {
        // 更新元が body でも、追加された部分だけを検索する。
        record.addedNodes.forEach(queueRoot);
        var anchor = record.target.nodeType === 1 && record.target.closest("a");
        if (anchor) queueRoot(anchor);
      }
    });
    schedule();
  });

  function observe() {
    if (document.body) {
      observer.observe(document.body, {
        childList: true, subtree: true, characterData: true,
        attributes: true, attributeFilter: ["href"],
      });
    }
  }

  function refresh() {
    // 自分の挿入で観測ループが回らないよう、描画中だけ監視を外す。
    observer.disconnect();
    try {
      ensureStyle();
      // プロフィール本体を先に処理し、同じユーザーのリンクへ重複して出さない。
      decorateProfile();
      if (fullRefresh) {
        decorateAnchors(document);
      } else {
        // 親子両方がキューにある場合、子を重ねて走査しない。
        dirtyRoots.forEach(function (root) {
          if (!root.isConnected) return;
          var parent = root.parentElement;
          while (parent) {
            if (dirtyRoots.has(parent)) return;
            parent = parent.parentElement;
          }
          decorateAnchors(root);
        });
      }
      fullRefresh = false;
      dirtyRoots.clear();
    } catch (err) {
      console.warn("[Libe Desk] お気に入りボタンの描画に失敗しました", err);
    }
    observe();
  }

  function schedule() {
    if (scheduled || document.hidden) return;
    scheduled = window.setTimeout(function () {
      scheduled = 0;
      if (!document.hidden) refresh();
    }, 120);
  }

  function applyList(list) {
    favoriteIds = {};
    if (Array.isArray(list)) {
      for (var i = 0; i < list.length; i++) {
        var entry = list[i];
        if (entry && typeof entry.id === "string") favoriteIds[entry.id] = true;
      }
    }
    repaint();
  }

  function patchHistory() {
    ["pushState", "replaceState"].forEach(function (name) {
      var original = window.history[name];
      if (typeof original !== "function") return;
      window.history[name] = function () {
        var result = original.apply(this, arguments);
        fullRefresh = true;
        schedule();
        return result;
      };
    });
    window.addEventListener("popstate", function () {
      fullRefresh = true;
      schedule();
    });
    document.addEventListener("visibilitychange", function () {
      if (!document.hidden) schedule();
    });
  }

  function start() {
    try {
      patchHistory();
      refresh();
      invoke("list_favorite_users")
        .then(applyList)
        .catch(function (err) {
          console.warn("[Libe Desk] お気に入りの読み込みに失敗しました", err);
        });
    } catch (err) {
      console.warn("[Libe Desk] お気に入り機能の初期化に失敗しました", err);
    }
  }

  // アプリ側の変更を受け取る口。イベント購読の権限をページへ渡さずに済ませる。
  window.__libeDeskFavorites = {
    apply: function (list) {
      try {
        applyList(list);
      } catch (err) {
        console.warn("[Libe Desk] お気に入りの反映に失敗しました", err);
      }
    },
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start, { once: true });
  } else {
    start();
  }
})();

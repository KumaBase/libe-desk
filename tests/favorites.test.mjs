import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const script = readFileSync(new URL("../src-tauri/src/inject/favorites.js", import.meta.url), "utf8");

async function fixture(t, html = "", path = "/") {
  const dom = new JSDOM(html, { url: "https://libecity.com" + path, runScripts: "outside-only", pretendToBeVisual: true });
  const observers = [];
  const NativeObserver = dom.window.MutationObserver;
  dom.window.MutationObserver = class extends NativeObserver {
    constructor(callback) { super(callback); observers.push(this); }
  };
  t.after(() => { observers.forEach(observer => observer.disconnect()); dom.window.close(); });
  const { window } = dom;
  const { document } = window;
  const calls = [];
  const timers = new Map();
  let nextTimer = 0;
  let hidden = false;
  Object.defineProperty(document, "hidden", { get: () => hidden });
  window.setTimeout = (fn) => { timers.set(++nextTimer, fn); return nextTimer; };
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => { calls.push({ cmd, args }); return []; },
  };
  window.eval(script);
  await new Promise(resolve => document.readyState === "loading"
    ? document.addEventListener("DOMContentLoaded", resolve, { once: true }) : resolve());
  async function flush() {
    await Promise.resolve();
    const pending = [...timers.values()];
    timers.clear();
    pending.forEach(fn => fn());
    await Promise.resolve();
  }
  await flush();
  return { window, document, calls, timers, flush,
    setHidden(value) { hidden = value; document.dispatchEvent(new window.Event("visibilitychange")); } };
}

test("large lists: unrelated updates never rescan the document or existing cards", async t => {
  const f = await fixture(t, '<main>' + Array.from({ length: 500 }, (_, i) =>
    '<article><a href="/user_profile/u' + i + '">User ' + i + '</a></article>').join("") + '</main><aside id="updates"></aside>');
  assert.equal(f.document.querySelectorAll(".libedesk-fav-btn").length, 500);
  let documentScans = 0;
  const query = f.document.querySelectorAll.bind(f.document);
  f.document.querySelectorAll = (...args) => { documentScans++; return query(...args); };
  const oldButton = f.document.querySelector(".libedesk-fav-btn");
  for (let i = 0; i < 5; i++) {
    f.document.getElementById("updates").append("new message");
    await f.flush();
  }
  assert.equal(documentScans, 0);
  assert.equal(f.document.querySelector(".libedesk-fav-btn"), oldButton);
  assert.equal(f.timers.size, 0, "our own DOM changes must not create an observer loop");
});

test("new subtrees, delayed text, replaced content and recycled hrefs get the correct button", async t => {
  const f = await fixture(t, '<main></main>');
  f.document.querySelector("main").innerHTML = '<article><a href="/user_profile/first"></a></article>';
  await f.flush();
  const anchor = f.document.querySelector("a");
  assert.equal(anchor.children.length, 0);
  anchor.append("First");
  await f.flush();
  assert.equal(anchor.querySelector("button").dataset.libedeskId, "first");
  anchor.setAttribute("href", "/user_profile/second");
  await f.flush();
  anchor.querySelector("button").click();
  await f.flush();
  assert.equal(f.calls.find(call => call.cmd === "add_favorite_user").args.id, "second");
  anchor.textContent = "Second";
  await f.flush();
  assert.equal(anchor.querySelectorAll("button").length, 1);
  anchor.href = "/room_list";
  await f.flush();
  assert.equal(anchor.querySelector("button"), null);
});

test("hidden changes are deferred and recovered when visible", async t => {
  const f = await fixture(t, "<main></main>");
  f.setHidden(true);
  f.document.querySelector("main").innerHTML = '<a href="/user_profile/hidden">Hidden</a>';
  await f.flush();
  assert.equal(f.document.querySelector("button"), null);
  assert.equal(f.timers.size, 0);
  f.setHidden(false);
  await f.flush();
  assert.equal(f.document.querySelector("button").dataset.libedeskId, "hidden");
});

test("SPA profile changes and replaced name nodes recreate the profile star", async t => {
  const f = await fixture(t, '<div class="user_profdata"><span class="username">First</span></div>', "/user_profile/first");
  assert.equal(f.document.querySelector("button").dataset.libedeskId, "first");
  f.window.history.pushState({}, "", "/user_profile/second");
  await f.flush();
  assert.equal(f.document.querySelector("button").dataset.libedeskId, "second");
  f.document.querySelector(".user_profdata").innerHTML = '<span class="username">Second</span>';
  await f.flush();
  assert.equal(f.document.querySelectorAll("button").length, 1);
  f.window.history.replaceState({}, "", "/");
  await f.flush();
  assert.equal(f.document.querySelector("button"), null);
});

test("favorite updates repaint existing stars without duplicating them", async t => {
  const f = await fixture(t, '<a href="/user_profile/first">First</a>');
  f.window.__libeDeskFavorites.apply([{ id: "first" }]);
  await f.flush();
  assert.equal(f.document.querySelector("button").getAttribute("aria-pressed"), "true");
  f.window.__libeDeskFavorites.apply([]);
  await f.flush();
  assert.equal(f.document.querySelector("button").getAttribute("aria-pressed"), "false");
  assert.equal(f.document.querySelectorAll("button").length, 1);
  assert.equal(f.timers.size, 0);
});

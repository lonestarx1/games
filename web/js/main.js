import { GAMES } from "./games.js";
import { getBest, setBest, getTheme, setTheme, hasSeenHowto, markHowtoSeen } from "./storage.js";

// Must match engine::GameState.
const STATE = { READY: 0, PLAYING: 1, PAUSED: 2, OVER: 3 };
const GAME_KEYS = new Set([
  "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space", "Enter",
  "KeyW", "KeyA", "KeyS", "KeyD", "KeyP", "Escape",
]);

const $ = (id) => document.getElementById(id);
const els = {
  menu: $("menu"), play: $("play"), list: $("game-list"), title: $("game-title"),
  score: $("score"), best: $("best"), canvas: $("canvas"), overlay: $("overlay"),
  overlayTitle: $("overlay-title"), overlaySub: $("overlay-sub"), help: $("help"),
  actions: $("actions"),
  themeToggle: $("theme-toggle"),
  howto: $("howto"), howtoOpen: $("howto-open"), howtoTitle: $("howto-title"),
  howtoGoal: $("howto-goal"), howtoRules: $("howto-rules"), howtoControls: $("howto-controls"),
  howtoTips: $("howto-tips"), howtoTipsTitle: $("howto-tips-title"), howtoOk: $("howto-ok"),
};

let theme = getTheme();
let session = null; // { meta, game, raf, best, lastState, lastScore, newBest }
const modules = new Map(); // id -> initialised wasm module

// ---------- theme ----------
function applyTheme(next) {
  theme = next;
  document.documentElement.dataset.theme = theme;
  setTheme(theme);
  session?.game.set_theme(theme);
}
els.themeToggle.addEventListener("click", () => applyTheme(theme === "retro" ? "modern" : "retro"));
applyTheme(theme);

// ---------- menu ----------
function renderMenu() {
  els.list.replaceChildren(...GAMES.map((g) => {
    const li = document.createElement("li");
    li.innerHTML = `
      <a class="card" href="#/${g.id}">
        <span class="card-art art-${g.id}" aria-hidden="true"></span>
        <span class="card-body">
          <span class="card-title"></span>
          <span class="card-blurb"></span>
          <span class="card-best">Best <b></b></span>
        </span>
      </a>
      <button class="card-help" type="button">How to play</button>`;
    li.querySelector(".card-title").textContent = g.title;
    li.querySelector(".card-blurb").textContent = g.blurb;
    li.querySelector(".card-best b").textContent = getBest(g.id);
    const help = li.querySelector(".card-help");
    help.setAttribute("aria-label", `How to play ${g.title}`);
    help.addEventListener("click", () => openHowto(g, false));
    return li;
  }));
}

// ---------- how to play ----------
function el(tag, text) {
  const e = document.createElement(tag);
  e.textContent = text;
  return e;
}

// `inGame` changes the button: from the menu it starts the game.
function openHowto(meta, inGame) {
  const h = meta.howto;
  if (!h) return;
  session?.game.pause();
  els.howtoTitle.textContent = meta.title;
  els.howtoGoal.textContent = h.goal;
  els.howtoRules.replaceChildren(...h.rules.map((r) => el("li", r)));
  els.howtoControls.replaceChildren(...h.controls.map(([keys, does]) => {
    const tr = document.createElement("tr");
    const th = el("th", keys);
    th.scope = "row";
    tr.append(th, el("td", does));
    return tr;
  }));
  const tips = h.tips ?? [];
  els.howtoTips.replaceChildren(...tips.map((t) => el("li", t)));
  els.howtoTips.hidden = els.howtoTipsTitle.hidden = tips.length === 0;
  els.howtoOk.textContent = inGame ? "Got it, let's play" : `Play ${meta.title}`;
  els.howto.dataset.game = inGame ? "" : meta.id;
  markHowtoSeen(meta.id);
  els.howto.showModal();
  els.howto.scrollTop = 0;
}

els.howtoOpen.addEventListener("click", () => session && openHowto(session.meta, true));
els.howto.addEventListener("close", () => {
  const id = els.howto.dataset.game;
  if (id && els.howto.returnValue === "ok") location.hash = `#/${id}`;
});
// Clicking the dimmed backdrop closes the card.
els.howto.addEventListener("click", (e) => { if (e.target === els.howto) els.howto.close(); });

// ---------- game session ----------
async function loadModule(meta) {
  if (!modules.has(meta.id)) {
    const mod = await import(meta.module);
    await mod.default(); // fetch + instantiate the .wasm
    modules.set(meta.id, mod);
  }
  return modules.get(meta.id);
}

function stopGame() {
  if (!session) return;
  cancelAnimationFrame(session.raf);
  session.game.free();
  session = null;
}

async function startGame(meta) {
  stopGame();
  els.title.textContent = meta.title;
  els.help.textContent = meta.help;
  els.canvas.classList.toggle("pointer", !!meta.pointer);
  renderActions(meta);
  els.howtoOpen.hidden = !meta.howto;
  showOverlay("Loading…", "");
  // First time playing this game: show the rules before anything else.
  if (meta.howto && !hasSeenHowto(meta.id)) openHowto(meta, true);

  let mod;
  try {
    mod = await loadModule(meta);
  } catch (err) {
    console.error(err);
    showOverlay("Couldn't load game", "Run scripts/build.sh, then reload.");
    return;
  }
  if (location.hash !== `#/${meta.id}`) return; // navigated away while loading

  const dpr = Math.min(window.devicePixelRatio || 1, 3);
  const ctx = els.canvas.getContext("2d");
  const game = new mod.Game(ctx, dpr, Math.floor(Math.random() * 2 ** 32));
  const w = game.width(), h = game.height();
  els.canvas.width = Math.round(w * dpr);
  els.canvas.height = Math.round(h * dpr);
  els.canvas.style.aspectRatio = `${w} / ${h}`;
  els.canvas.parentElement.style.setProperty("--aspect", `${w / h}`);
  game.set_theme(theme);

  const best = getBest(meta.id);
  session = { meta, game, raf: 0, best, lastState: -1, lastScore: -1, newBest: false };
  els.best.textContent = best;

  let last = performance.now();
  const frame = (now) => {
    if (!session || session.game !== game) return;
    const dt = Math.min(now - last, 100); // don't jump after a stall
    last = now;
    game.update(dt);
    game.render();
    syncHud();
    session.raf = requestAnimationFrame(frame);
  };
  session.raf = requestAnimationFrame(frame);
}

function syncHud() {
  const s = session;
  const score = s.game.score();
  const state = s.game.state();

  if (score !== s.lastScore) {
    s.lastScore = score;
    els.score.textContent = score;
    if (score > s.best) {
      s.best = score;
      s.newBest = true;
      setBest(s.meta.id, score);
      els.best.textContent = score;
    }
  }
  if (state !== s.lastState) {
    if (state === STATE.PLAYING && s.lastState === STATE.OVER) s.newBest = false;
    s.lastState = state;
    switch (state) {
      case STATE.READY: showOverlay(s.meta.title, s.meta.start); break;
      case STATE.PAUSED: showOverlay("Paused", "Press Space to resume"); break;
      case STATE.OVER: {
        const custom = s.game.end_message();
        const title = custom ?? (s.newBest ? "New best!" : "Game over");
        const best = custom && s.newBest ? " · New best!" : "";
        showOverlay(title, `Score ${score}${best} · Press Space to play again`);
        break;
      }
      default: hideOverlay();
    }
  }
}

function showOverlay(title, sub) {
  els.overlayTitle.textContent = title;
  els.overlaySub.textContent = sub;
  els.overlay.hidden = false;
}
function hideOverlay() { els.overlay.hidden = true; }

// ---------- input ----------
window.addEventListener("keydown", (e) => {
  if (!session || els.howto.open || e.metaKey || e.ctrlKey || e.altKey) return;
  if (GAME_KEYS.has(e.code)) e.preventDefault();
  if (e.repeat) return; // held keys are tracked via keydown/keyup pairs
  session.game.key_down(e.code);
});
window.addEventListener("keyup", (e) => {
  if (session && !els.howto.open) session.game.key_up(e.code);
});
// Mouse / touch, converted to the game's logical coordinates.
function toLogical(e) {
  const r = els.canvas.getBoundingClientRect();
  return [
    (e.clientX - r.left) * (session.game.width() / r.width),
    (e.clientY - r.top) * (session.game.height() / r.height),
  ];
}
// Right-click sends pointer_alt. On touch, games with `longPress` get a tap
// on release, and a press held still for LONG_PRESS_MS becomes pointer_alt.
const LONG_PRESS_MS = 450;
let press = null; // pending touch press: { game, x, y, cx, cy, timer }
function cancelPress() {
  if (press) clearTimeout(press.timer);
  press = null;
}
els.canvas.addEventListener("contextmenu", (e) => { if (session?.meta.pointer) e.preventDefault(); });
els.canvas.addEventListener("pointerdown", (e) => {
  if (!session) return;
  const [x, y] = toLogical(e);
  if (e.button === 2) {
    if (session.game.pointer_alt(x, y)) e.preventDefault();
    return;
  }
  if (e.button !== 0) return;
  if (e.pointerType === "touch" && session.meta.longPress) {
    e.preventDefault();
    cancelPress();
    const game = session.game;
    press = {
      game, x, y, cx: e.clientX, cy: e.clientY,
      timer: setTimeout(() => {
        press = null;
        if (session?.game === game && game.pointer_alt(x, y)) navigator.vibrate?.(25);
      }, LONG_PRESS_MS),
    };
    return;
  }
  if (session.game.pointer_down(x, y)) {
    e.preventDefault();
    // Keep receiving moves and the release even if a drag leaves the canvas.
    els.canvas.setPointerCapture?.(e.pointerId);
  }
});
els.canvas.addEventListener("pointerup", (e) => {
  if (!session) return;
  if (!press) {
    if (e.button === 0) session.game.pointer_up(...toLogical(e));
    return;
  }
  const { game, x, y } = press;
  cancelPress();
  if (session?.game === game) game.pointer_down(x, y);
});
els.canvas.addEventListener("pointercancel", cancelPress);
els.canvas.addEventListener("pointermove", (e) => {
  if (press && Math.hypot(e.clientX - press.cx, e.clientY - press.cy) > 12) cancelPress();
  if (session) session.game.pointer_move(...toLogical(e));
});
els.canvas.addEventListener("pointerleave", () => { if (session) session.game.pointer_move(-1, -1); });
// Clicking the overlay acts like Space (start / resume / play again).
els.overlay.addEventListener("click", () => {
  if (!session) return;
  session.game.key_down("Space");
  session.game.key_up("Space");
});

// On-screen buttons that send a key to the game (e.g. Hint, Undo).
function renderActions(meta) {
  els.actions.replaceChildren(...(meta.actions ?? []).map(({ label, code }) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "action";
    b.textContent = label;
    b.addEventListener("mousedown", (e) => e.preventDefault()); // keep focus off, so Space stays a game key
    b.addEventListener("click", () => {
      if (!session) return;
      session.game.key_down(code);
      session.game.key_up(code);
    });
    return b;
  }));
}

// Pause when the player looks away.
document.addEventListener("visibilitychange", () => { if (document.hidden) session?.game.pause(); });
window.addEventListener("blur", () => session?.game.pause());

// ---------- routing ----------
function route() {
  const id = location.hash.replace(/^#\/?/, "");
  const meta = GAMES.find((g) => g.id === id);
  els.menu.hidden = !!meta;
  els.play.hidden = !meta;
  if (els.howto.open && els.howto.dataset.game !== meta?.id) els.howto.close();
  if (meta) {
    startGame(meta);
  } else {
    stopGame();
    renderMenu();
  }
}
window.addEventListener("hashchange", route);
route();

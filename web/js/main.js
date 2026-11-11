import { GAMES } from "./games.js";
import { getBest, setBest, getTheme, setTheme } from "./storage.js";

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
      </a>`;
    li.querySelector(".card-title").textContent = g.title;
    li.querySelector(".card-blurb").textContent = g.blurb;
    li.querySelector(".card-best b").textContent = getBest(g.id);
    return li;
  }));
}

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
  showOverlay("Loading…", "");

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
  if (!session || e.metaKey || e.ctrlKey || e.altKey) return;
  if (GAME_KEYS.has(e.code)) e.preventDefault();
  if (e.repeat) return; // held keys are tracked via keydown/keyup pairs
  session.game.key_down(e.code);
});
window.addEventListener("keyup", (e) => {
  if (session) session.game.key_up(e.code);
});
// Mouse / touch, converted to the game's logical coordinates.
function toLogical(e) {
  const r = els.canvas.getBoundingClientRect();
  return [
    (e.clientX - r.left) * (session.game.width() / r.width),
    (e.clientY - r.top) * (session.game.height() / r.height),
  ];
}
els.canvas.addEventListener("pointerdown", (e) => {
  if (!session) return;
  if (session.game.pointer_down(...toLogical(e))) e.preventDefault();
});
els.canvas.addEventListener("pointermove", (e) => { if (session) session.game.pointer_move(...toLogical(e)); });
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
  if (meta) {
    startGame(meta);
  } else {
    stopGame();
    renderMenu();
  }
}
window.addEventListener("hashchange", route);
route();

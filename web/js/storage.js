// localStorage can be unavailable (private mode, blocked storage), so every
// access is wrapped and falls back to sensible defaults.

function read(key) {
  try { return localStorage.getItem(key); } catch { return null; }
}

function write(key, value) {
  try { localStorage.setItem(key, String(value)); } catch { /* ignore */ }
}

export function getBest(gameId) {
  const n = Number.parseInt(read(`arcade:best:${gameId}`) ?? "0", 10);
  return Number.isFinite(n) ? n : 0;
}

export function setBest(gameId, score) {
  write(`arcade:best:${gameId}`, score);
}

export function getTheme() {
  return read("arcade:theme") === "retro" ? "retro" : "modern";
}

export function setTheme(theme) {
  write("arcade:theme", theme);
}

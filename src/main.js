// Waktu popover frontend — vanilla JS, no bundler. Binds to the Rust
// commands via the window.__TAURI__ global (app.withGlobalTauri: true).

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const MALAY_DAYS = {
  Monday: "Isnin", Tuesday: "Selasa", Wednesday: "Rabu", Thursday: "Khamis",
  Friday: "Jumaat", Saturday: "Sabtu", Sunday: "Ahad",
};
const MALAY_MONTHS = {
  January: "Januari", February: "Februari", March: "Mac", April: "April",
  May: "Mei", June: "Jun", July: "Julai", August: "Ogos",
  September: "September", October: "Oktober", November: "November", December: "Disember",
};
// Hijri months in API-month order (1-12), Malay/Arabic-transliteration style per spec.
const HIJRI_MONTHS = [
  "Muharram", "Safar", "Rabiulawal", "Rabiulakhir", "Jamadilawal", "Jamadilakhir",
  "Rejab", "Syaaban", "Ramadan", "Syawal", "Zulkaedah", "Zulhijjah",
];

let state = null;

// Backend formats gregorian as "%A, %d %B %Y" (English) — translate to Malay.
function formatMalayGregorian(greg) {
  const m = greg.match(/^(\w+),\s*(\d+)\s+(\w+)\s+(\d+)$/);
  if (!m) return greg;
  const [, day, date, month, year] = m;
  const malayDay = MALAY_DAYS[day] || day;
  const malayMonth = MALAY_MONTHS[month] || month;
  return `${malayDay}, ${parseInt(date, 10)} ${malayMonth} ${year}`;
}

// Backend hijri is raw "YYYY-MM-DD" — render as "DD MonthName YYYY".
function formatMalayHijri(hijri) {
  const m = hijri.match(/^(\d+)-(\d+)-(\d+)$/);
  if (!m) return hijri;
  const [, year, month, day] = m;
  const name = HIJRI_MONTHS[parseInt(month, 10) - 1] || month;
  return `${parseInt(day, 10)} ${name} ${year}`;
}

function formatTime(atMs) {
  const d = new Date(atMs);
  let h = d.getHours();
  const m = d.getMinutes();
  const ampm = h >= 12 ? "PM" : "AM";
  h = h % 12 || 12;
  return `${h}:${String(m).padStart(2, "0")} ${ampm}`;
}

function arabicForName(name) {
  const row = state?.prayers.find((r) => r.name === name);
  return row ? row.arabic : "";
}

function render(newState) {
  state = newState;

  document.getElementById("hero-name").textContent = state.next.name;
  document.getElementById("hero-arabic").textContent = arabicForName(state.next.name);
  document.getElementById("greg-date").textContent = formatMalayGregorian(state.gregorian);
  document.getElementById("hijri-date").textContent = formatMalayHijri(state.hijri);
  document.getElementById("qibla-deg").textContent = `${Math.round(state.qibla_deg)}°`;
  document.getElementById("zone-district").textContent = `${state.zone} · ${state.district}`;

  const list = document.getElementById("prayer-list");
  list.innerHTML = "";
  for (const row of state.prayers) {
    const el = document.createElement("div");
    el.className = ["row", row.passed && "passed", row.is_active && "active"]
      .filter(Boolean)
      .join(" ");
    el.innerHTML = `
      <span class="tick"></span>
      <span class="names"><span class="name">${row.name}</span><span class="arabic">${row.arabic}</span></span>
      <span class="time">${formatTime(row.at_ms)}</span>
    `;
    list.appendChild(el);
  }

  tickCountdown();
}

// 1s local tick: recomputes the hero pill from next.at_ms without hitting the backend.
function tickCountdown() {
  if (!state) return;
  const diffMs = state.next.at_ms - Date.now();
  const totalMin = Math.max(0, Math.round(diffMs / 60000));
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  const label = h > 0 ? `${h} h ${m} min` : `${m} min`;
  document.getElementById("countdown-val").textContent = label;
  document.getElementById("countdown-at").textContent = `· ${formatTime(state.next.at_ms)}`;
}
setInterval(tickCountdown, 1000);

// --- settings panel ---

function populateZoneSelect(zones) {
  const select = document.getElementById("zone-select");
  select.innerHTML = "";

  const auto = document.createElement("option");
  auto.value = "";
  auto.textContent = "Auto (guna lokasi)";
  select.appendChild(auto);

  const byNegeri = new Map();
  for (const z of zones) {
    if (!byNegeri.has(z.negeri)) byNegeri.set(z.negeri, []);
    byNegeri.get(z.negeri).push(z);
  }
  for (const [negeri, list] of byNegeri) {
    const group = document.createElement("optgroup");
    group.label = negeri;
    for (const z of list) {
      const opt = document.createElement("option");
      opt.value = z.jakimCode;
      opt.textContent = `${z.jakimCode} · ${z.daerah}`;
      group.appendChild(opt);
    }
    select.appendChild(group);
  }

  select.value = state?.zone || "";
}

async function openSettings() {
  document.getElementById("settings-panel").hidden = false;
  try {
    const zones = await invoke("list_zones");
    populateZoneSelect(zones);
  } catch (e) {
    console.error("list_zones failed", e);
  }
}

function closeSettings() {
  document.getElementById("settings-panel").hidden = true;
}

document.getElementById("settings-gear").addEventListener("click", openSettings);
document.getElementById("settings-close").addEventListener("click", closeSettings);
document.getElementById("zone-select").addEventListener("change", async (e) => {
  const code = e.target.value;
  try {
    const next = code ? await invoke("set_zone", { code }) : await invoke("clear_zone");
    render(next);
    closeSettings();
  } catch (err) {
    console.error("zone change failed", err);
  }
});

// --- theme (follows macOS appearance) ---

function applyTheme() {
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
  drawGirih();
}
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);

// --- girih 8-point star field behind the hero ---

function drawGirih() {
  const c = document.getElementById("girih");
  if (!c) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = c.clientWidth, h = c.clientHeight;
  c.width = w * dpr; c.height = h * dpr;
  const ctx = c.getContext("2d");
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);
  const gold = getComputedStyle(document.documentElement).getPropertyValue("--gold").trim();
  ctx.strokeStyle = gold || "#C9A34E";
  ctx.lineWidth = 0.6;
  ctx.globalAlpha = 0.10;
  const R = 34, step = R * 1.62;
  for (let y = -step; y < h + step; y += step) {
    for (let x = -step; x < w + step; x += step) {
      const ox = x + (Math.round(y / step) % 2 ? step / 2 : 0);
      drawStar(ctx, ox, y, R);
    }
  }
}

function drawStar(ctx, cx, cy, R) {
  const points = 8, inner = R * 0.44;
  ctx.beginPath();
  for (let i = 0; i < points * 2; i++) {
    const ang = (Math.PI / points) * i - Math.PI / 2;
    const rad = i % 2 === 0 ? R : inner;
    const px = cx + Math.cos(ang) * rad;
    const py = cy + Math.sin(ang) * rad;
    i === 0 ? ctx.moveTo(px, py) : ctx.lineTo(px, py);
  }
  ctx.closePath();
  ctx.stroke();
  ctx.beginPath();
  for (let i = 0; i < points; i++) {
    const ang = ((2 * Math.PI) / points) * i - Math.PI / 8;
    const px = cx + Math.cos(ang) * inner;
    const py = cy + Math.sin(ang) * inner;
    i === 0 ? ctx.moveTo(px, py) : ctx.lineTo(px, py);
  }
  ctx.closePath();
  ctx.stroke();
}
window.addEventListener("resize", drawGirih);

// --- bootstrap ---

applyTheme();
drawGirih();

invoke("get_state")
  .then(render)
  .catch((e) => console.error("get_state failed", e));

listen("state-updated", (event) => render(event.payload));

// The webview is suspended while the panel is hidden, so it can miss
// `state-updated` events and go stale (e.g. show yesterday after a day rollover).
// Re-fetch fresh state every time the popover is opened.
listen("popover-shown", () => {
  invoke("get_state")
    .then(render)
    .catch((e) => console.error("get_state on show failed", e));
});

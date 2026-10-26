// Card Place — card page. Vanilla JS, no dependencies.
//
// Data sources:
//  - data/zekrom.json        : card, sales history, population, market table (sample data)
//  - agent (localhost:3000)  : on-chain record via GET /vault/:code (when the agent runs)
//  - Horizon testnet         : order book for the card asset against XLM (public API)

const AGENT_URL = "http://localhost:3000";
const HORIZON = "https://horizon-testnet.stellar.org";
const ISSUER = "GBNZP4YND7GXOM26YNBOAXIMTEMKNHDX7CQ5VZOJ3VKW7HDJSQR3XK75";
const CONTRACT = "CDN5OOWTOYKEXMH5CDG7APQRQEHGFRKWQEAHQR5XR3643YOM2YEMEMRO";

const $ = (sel) => document.querySelector(sel);
const fmt = (n) => n.toLocaleString("en-GB", { maximumFractionDigits: 0 });
const fmtDate = (iso) => new Date(iso).toLocaleDateString("en-GB");

let DATA;
const state = { grader: "PSA", grade: "10", period: "all" }; // grader fixed to the card's grader

init();

async function init() {
  DATA = await (await fetch("data/zekrom.json")).json();
  selectTab($('#market-tabs button[data-cat="cap"]'));
  renderPopulation(DATA.population);
  $("#details").innerHTML = DATA.details.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join("");
  bindControls();
  bindCardControls();
  renderSales();
  loadVault();      // on-chain, best effort
  loadOrderBook();  // Horizon, best effort
}

/* ---------- Rarity ---------- */
function renderPopulation(pop) {
  const current = pop.by_grade.find((g) => g.grade === DATA.grade);
  const share = current ? Math.round((current.count / pop.total) * 100) : 0;
  const rank = [...pop.by_grade].sort((a, b) => a.count - b.count).findIndex((g) => g.grade === DATA.grade) + 1;
  const scarcity = rank === 1 ? "the scarcest grade" : rank === pop.by_grade.length ? "the most common grade" : "a mid-range grade";
  $("#rarity-big").textContent = current ? `1 of ${current.count}` : "—";
  $("#rarity-sub").textContent = `${pop.grader} ${DATA.grade} cards in the world`;
  $("#rarity-gem").textContent = `${share}%`;       // share of copies that came back a 10
  $("#rarity-total").textContent = pop.total;
  $("#rarity-text").textContent = current
    ? `${pop.total} copies of this card have been graded by ${pop.grader} worldwide. ${current.count} of them are ${pop.grader} ${current.grade} (${share}%), ${scarcity} for this card.`
    : "";
  $("#pop").innerHTML = pop.by_grade.map((g) => `
    <div class="grade ${g.grade === DATA.grade ? "current" : ""}">
      <div class="grade-count">${g.count}</div>
      <div class="grade-name">${pop.grader} ${g.grade}</div>
      <div class="grade-share">${Math.round((g.count / pop.total) * 100)}%</div>
    </div>`).join("");
  const more = $("#pop-more"), toggle = $("#pop-toggle");
  toggle.addEventListener("click", () => {
    more.hidden = !more.hidden;
    toggle.textContent = more.hidden ? "Click to see more data" : "Hide";
  });
}

/* ---------- Sales history + chart ---------- */
function bindControls() {
  $("#market-tabs").addEventListener("click", (e) => {
    const b = e.target.closest("button"); if (!b) return;
    selectTab(b);
  });
}

// The Zekrom page lives under the Example tab (it is the only card with a page).
// Pokémon and One Piece have no data yet: they only show a message.
const GAMES = ["Pokémon", "One Piece"];
function selectTab(b) {
    $("#market-tabs .active")?.classList.remove("active"); b.classList.add("active");
    const cat = b.dataset.cat, example = cat === "example", game = GAMES.includes(cat);
    $("#card").hidden = !example;
    $("#explore").hidden = !game; $("#market").hidden = cat !== "cap";
    if (cat === "cap") return showMarketCap();
    document.body.classList.remove("wide");
    if (game) $("#explore-title").textContent = `${cat} data is not available yet`;
}

function openCard() {
  selectTab($('#market-tabs button[data-cat="example"]'));
  $("#card").scrollIntoView({ behavior: "smooth" });
}

function bindCardControls() {
  $("#periods").addEventListener("click", (e) => {
    const b = e.target.closest("button"); if (!b) return;
    state.period = b.dataset.days;
    $("#periods .active")?.classList.remove("active"); b.classList.add("active");
    renderSales();
  });
  $("#grade-filter").addEventListener("change", (e) => { state.grade = e.target.value; renderSales(); });
  // turn the card: the slider or a drag drives the rotation, it snaps to the nearest face on release
  const inner = $("#card3d-inner"), turn = $("#turn"), card = $("#card3d");
  let angle = 0;
  const render = (animate) => {
    inner.style.transition = animate ? "transform .6s cubic-bezier(0.4,0,0.2,1)" : "none";
    inner.style.transform = `rotateY(${angle}deg)`;
    turn.value = angle;
  };
  turn.addEventListener("input", () => { angle = Number(turn.value); render(false); });
  let dragX = null;
  card.addEventListener("pointerdown", (e) => { dragX = e.clientX; });
  window.addEventListener("pointermove", (e) => {
    if (dragX !== null) { angle = Math.max(0, Math.min(180, angle + (e.clientX - dragX) * 0.8)); dragX = e.clientX; render(false); }
  });
  window.addEventListener("pointerup", () => { if (dragX === null) return; dragX = null; angle = angle > 90 ? 180 : 0; render(true); });
  render(false);
  $("#btn-buy").addEventListener("click", () => alert("Buy now at the asking price on the native Stellar order book: token against USDC, one atomic transaction, about five seconds."));
  $("#btn-offer").addEventListener("click", () => alert("Place a buy offer at your price. It sits on the public order book until a seller accepts, or until you cancel it."));
  $("#btn-sell").addEventListener("click", () => alert("Place a sell order on the native Stellar order book. The card stays in the vault."));
}

function filteredSales() {
  let s = DATA.sales.filter((x) => (state.grader === "all" || x.grader === state.grader) && (state.grade === "all" || x.grade === state.grade));
  s.sort((a, b) => a.date.localeCompare(b.date));
  if (!s.length) return s;
  const end = new Date(s[s.length - 1].date);
  let start = null;
  if (state.period === "ytd") start = new Date(end.getFullYear(), 0, 1);
  else if (state.period !== "all") start = new Date(end.getTime() - Number(state.period) * 86400e3);
  return start ? s.filter((x) => new Date(x.date) >= start) : s;
}

function renderSales() {
  const s = filteredSales();
  const last = s[s.length - 1]?.price, first = s[0]?.price;
  $("#st-last").textContent = last ? fmt(last) : "—";
  $("#st-record").textContent = s.length ? fmt(Math.max(...s.map((x) => x.price))) : "—";
  const trend = first && last ? ((last - first) / first) * 100 : null;
  $("#st-trend").textContent = trend === null ? "—" : (trend >= 0 ? "+" : "") + trend.toFixed(0);
  $("#st-trend-wrap").classList.toggle("down", trend !== null && trend < 0);
  $("#x-start").textContent = s.length ? fmtDate(s[0].date) : "";
  $("#x-end").textContent = s.length ? fmtDate(s[s.length - 1].date) : "";
  drawChart(s);
}

function drawChart(points) {
  const svg = $("#chart"); const tip = $("#chart-tip");
  svg.innerHTML = ""; tip.hidden = true;
  if (points.length < 2) { svg.innerHTML = `<text x="450" y="150" fill="#86868c" text-anchor="middle" font-size="14">Not enough sales in this period</text>`; return; }

  const W = 900, H = 300, padL = 52, padR = 12, padT = 16, padB = 20;
  const prices = points.map((p) => p.price);
  const min = Math.floor(Math.min(...prices) / 100) * 100, max = Math.ceil(Math.max(...prices) / 100) * 100;
  const t0 = new Date(points[0].date).getTime(), t1 = new Date(points[points.length - 1].date).getTime();
  const X = (t) => padL + ((t - t0) / Math.max(1, t1 - t0)) * (W - padL - padR);
  const Y = (p) => padT + (1 - (p - min) / Math.max(1, max - min)) * (H - padT - padB);
  const pts = points.map((p) => [X(new Date(p.date).getTime()), Y(p.price), p]);

  // smooth curve (Catmull-Rom → Bézier)
  let d = `M ${pts[0][0]} ${pts[0][1]}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] || pts[i], p1 = pts[i], p2 = pts[i + 1], p3 = pts[i + 2] || p2;
    d += ` C ${p1[0] + (p2[0] - p0[0]) / 6} ${p1[1] + (p2[1] - p0[1]) / 6}, ${p2[0] - (p3[0] - p1[0]) / 6} ${p2[1] - (p3[1] - p1[1]) / 6}, ${p2[0]} ${p2[1]}`;
  }
  const area = `${d} L ${pts[pts.length - 1][0]} ${H - padB} L ${pts[0][0]} ${H - padB} Z`;

  let grid = "";
  for (let i = 0; i <= 2; i++) {
    const v = min + ((max - min) * i) / 2, y = Y(v);
    grid += `<line x1="${padL}" x2="${W - padR}" y1="${y}" y2="${y}" stroke="rgba(255,255,255,0.07)" stroke-dasharray="3 5"/>
             <text x="${padL - 10}" y="${y + 4}" fill="#86868c" font-size="12" text-anchor="end">€${fmt(v)}</text>`;
  }

  svg.innerHTML = `
    <defs><linearGradient id="fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#fff" stop-opacity="0.16"/><stop offset="100%" stop-color="#fff" stop-opacity="0"/>
    </linearGradient></defs>
    ${grid}
    <path d="${area}" fill="url(#fill)"/>
    <path d="${d}" fill="none" stroke="#fff" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"
          style="stroke-dasharray:2000;stroke-dashoffset:2000;animation:draw 1.2s cubic-bezier(0.4,0,0.2,1) forwards"/>
    ${pts.map(([x, y]) => `<circle cx="${x}" cy="${y}" r="3.5" fill="#0e0e0f" stroke="#fff" stroke-width="2"/>`).join("")}
    <style>@keyframes draw{to{stroke-dashoffset:0}}</style>`;

  svg.onmousemove = (e) => {
    const r = svg.getBoundingClientRect();
    const mx = ((e.clientX - r.left) / r.width) * W;
    let best = pts[0];
    for (const p of pts) if (Math.abs(p[0] - mx) < Math.abs(best[0] - mx)) best = p;
    tip.hidden = false;
    tip.innerHTML = `<b>€${fmt(best[2].price)}</b> · ${best[2].grader} ${best[2].grade}<br><span style="color:#86868c">${fmtDate(best[2].date)}</span>`;
    tip.style.left = (best[0] / W) * r.width + "px";
    tip.style.top = (best[1] / H) * r.height + "px";
  };
  svg.onmouseleave = () => (tip.hidden = true);
}

/* ---------- Card record (on-chain) ---------- */
async function ledgerDate(seq) {
  try { return fmtDate((await (await fetch(`${HORIZON}/ledgers/${seq}`)).json()).closed_at); } catch { return null; }
}

async function loadVault() {
  const text = $("#record-text"), tag = $("#vault-status");
  $("#btn-track").href = `https://stellar.expert/explorer/testnet/asset/${DATA.asset_code}-${ISSUER}`;
  try {
    const r = await fetch(`${AGENT_URL}/vault/${DATA.asset_code}`);
    if (!r.ok) throw new Error(r.status);
    const c = await r.json();
    const since = await ledgerDate(c.registered_at);
    const label = { InVault: "In vault", RedeemRequested: "Leaving the vault", Shipped: "Shipped" }[c.status] || c.status;
    tag.textContent = label; tag.className = "tag " + (c.status === "InVault" ? "ok" : "warn");
    const id = `${c.grader} ${c.cert}`;
    text.textContent = c.status === "InVault"
      ? `This card (${id}) has been in our vault since ${since ?? "ledger " + c.registered_at}. You can track it here.`
      : c.status === "RedeemRequested"
      ? `This card (${id}) entered our vault on ${since ?? "ledger " + c.registered_at}. Its owner has asked for it to be shipped. You can track it here.`
      : `This card (${id}) was in our vault from ${since ?? "ledger " + c.registered_at} and has been shipped to its owner${c.tracking ? ` (tracking ${c.tracking})` : ""}. You can track it here.`;
  } catch {
    tag.textContent = "service offline"; tag.className = "tag";
    text.textContent = `This card (${DATA.grader} ${DATA.cert}) is registered in our vault. Start the agent (npm start) to read the live record. You can track it here.`;
  }
}

/* ---------- Buy now price (Horizon order book) ---------- */
async function xlmEurRate() {
  try {
    const r = await fetch("https://api.coingecko.com/api/v3/simple/price?ids=stellar&vs_currencies=eur");
    return (await r.json()).stellar.eur;
  } catch { return null; }
}

async function loadOrderBook() {
  const btn = $("#btn-buy");
  try {
    const q = new URLSearchParams({
      selling_asset_type: "credit_alphanum12", selling_asset_code: DATA.asset_code, selling_asset_issuer: ISSUER,
      buying_asset_type: "native", limit: "1",
    });
    const [ob, rate] = await Promise.all([(await fetch(`${HORIZON}/order_book?${q}`)).json(), xlmEurRate()]);
    const ask = ob.asks[0] && Number(ob.asks[0].price);
    if (!ask) { btn.textContent = "No seller right now"; return; }
    btn.textContent = rate ? `Buy now for €${fmt(ask * rate)}` : `Buy now for ${fmt(ask)} XLM`;
    loadTransactions(rate);
  } catch {
    btn.textContent = "Buy now";
    loadTransactions(null);
  }
}

/* ---------- Recent transactions ---------- */
const daysAgo = (iso, ref) => Math.round((ref - new Date(iso)) / 86400e3);
const ago = (iso, ref) => { const d = daysAgo(iso, ref); return d < 1 ? "today" : d === 1 ? "yesterday" : d < 30 ? `${d} days ago` : d < 365 ? `${Math.max(1, Math.round(d / 30))} month${Math.round(d / 30) > 1 ? "s" : ""} ago` : fmtDate(iso); };

async function loadTransactions(rate) {
  const el = $("#txs");
  const rows = [];

  // real on-chain trades of the card token, if any
  try {
    const q = new URLSearchParams({
      base_asset_type: "credit_alphanum12", base_asset_code: DATA.asset_code, base_asset_issuer: ISSUER,
      counter_asset_type: "native", order: "desc", limit: "5",
    });
    const t = await (await fetch(`${HORIZON}/trades?${q}`)).json();
    for (const tr of t._embedded.records) {
      const xlm = Number(tr.counter_amount) / Number(tr.base_amount);
      rows.push({ date: tr.ledger_close_time, price: rate ? xlm * rate : null, xlm, grade: `${DATA.grader} ${DATA.grade}`,
        buyer: tr.base_is_seller ? tr.counter_account : tr.base_account, onchain: true });
    }
  } catch { /* Horizon unreachable: sample data only */ }

  // sample sales
  const ref = new Date(DATA.sales.map((x) => x.date).sort().at(-1));
  for (const x of [...DATA.sales].filter((x) => x.grader === DATA.grader && x.grade === DATA.grade).sort((a, b) => b.date.localeCompare(a.date)).slice(0, 6)) {
    rows.push({ date: x.date, price: x.price, grade: `${x.grader} ${x.grade}`, buyer: x.buyer, onchain: false });
  }

  const now = rows.some((r) => r.onchain) ? new Date() : ref;
  el.innerHTML = `<div class="tx tx-head"><div>Price</div><div>Grade</div><div>When</div><div>Buyer</div></div>` + rows.map((r) => `
    <div class="tx">
      <div class="tx-price">${r.price != null ? "€" + fmt(r.price) : fmt(r.xlm) + " XLM"}</div>
      <div class="tx-grade"><span class="grade-chip">${r.grade}</span></div>
      <div class="tx-when">${ago(r.date, now)}</div>
      <div class="tx-buyer">${r.onchain ? `<span class="mono">${r.buyer.slice(0, 4)}…${r.buyer.slice(-4)}</span><span class="live-dot" title="On-chain"></span>` : r.buyer}</div>
    </div>`).join("");
}

/* ---------- Market cap (overview by game + top cards) ---------- */
let CAP, OV;
async function showMarketCap() {
  CAP = CAP || await (await fetch("data/market-cap.json")).json();
  OV = OV || await (await fetch("data/market-overview.json")).json();
  $("#market-title").textContent = "Market cap";
  $("#cap-bar").hidden = false;
  renderOverview();
  renderMarketCap();
  setCapView(capView);
}

// "By TCG" shows the overview by game, "By card" the top cards table with its game filter
let capView = "tcg";
function setCapView(view) {
  capView = view;
  $("#cap-view .active")?.classList.remove("active");
  $(`#cap-view button[data-view="${view}"]`).classList.add("active");
  $("#overview").hidden = view !== "tcg";
  document.body.classList.add("wide");
  $("#cap-filter").hidden = view !== "cards";
  $("#cap-grid").hidden = view !== "cards";
  $("#market-sub").textContent = view === "tcg" ? "Every TCG, ranked by market value index. Click a column to sort." : "Top graded cards across every TCG. Pick several games to mix them, click a column to sort.";
}
$("#cap-view").addEventListener("click", (e) => { const b = e.target.closest("button"); if (b) setCapView(b.dataset.view); });

const pct = (v) => `<td class="num ${v >= 0 ? "up" : "down"}">${v >= 0 ? "" : ""}${v.toFixed(2)}% <span class="arw">${v >= 0 ? "▲" : "▼"}</span></td>`;
function spark(pts, up) {
  const min = Math.min(...pts), max = Math.max(...pts), w = 120, h = 34;
  const d = pts.map((p, i) => `${(i / (pts.length - 1) * w).toFixed(1)},${(h - 2 - (p - min) / (max - min || 1) * (h - 4)).toFixed(1)}`).join(" ");
  return `<svg class="spark ${up ? "up" : "down"}" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none"><polyline points="${d}" /></svg>`;
}

// sortable columns: click a header to sort by it, click again to flip the order
const SORT = { ov: { key: "rank", dir: 1 }, cap: { key: "rank", dir: 1 } };
function sorted(rows, { key, dir }) {
  return [...rows].sort((a, b) => (typeof a[key] === "string" ? a[key].localeCompare(b[key]) : a[key] - b[key]) * dir);
}
function markSort(table, { key, dir }) {
  table.querySelectorAll("th.sort").forEach((th) => th.dataset.dir = th.dataset.sort === key ? (dir > 0 ? "asc" : "desc") : "");
}
function bindSort(table, which, render) {
  table.querySelector("thead").addEventListener("click", (e) => {
    const th = e.target.closest("th.sort"); if (!th) return;
    const s = SORT[which], k = th.dataset.sort;
    // numbers start high-to-low, names and rank start low-to-high
    s.dir = s.key === k ? -s.dir : (k === "rank" || k === "name" ? 1 : -1);
    s.key = k; render();
  });
}

function renderOverview() {
  $("#ov-table tbody").innerHTML = sorted(OV.games, SORT.ov).map((g) => `<tr>
      <td class="num rank">${g.rank}</td>
      <td><div class="row-name"><span class="game-logo"><img src="${g.logo}" alt="${g.name}"></span><div class="row-title">${g.name}</div></div></td>
      <td class="num strong">$${fmt(g.index)}</td>${pct(g.d7)}${pct(g.d30)}${pct(g.d90)}
      <td class="num">${g.entry}</td><td class="num">${g.momentum}</td><td class="num">${g.dominance.toFixed(2)}%</td><td class="num">${fmt(g.cards)}</td>
      <td class="num">${spark(g.spark, g.d90 >= 0)}</td></tr>`).join("");
  markSort($("#ov-table"), SORT.ov);
  $("#ov-note").textContent = OV.note;
}

// game filter: several games can be picked at once to mix them (e.g. Pokémon + Yu-Gi-Oh!); none picked = all
const capGames = new Set();
// game chips above the card table: several can be picked at once to mix games; none picked = all
function renderCapFilter() {
  const games = [...new Set(CAP.cards.map((c) => c.game))];
  $("#cap-filter").innerHTML = `<button data-game="all" class="${capGames.size ? "" : "active"}">All</button>` +
    games.map((g) => `<button data-game="${g}" class="${capGames.has(g) ? "active" : ""}">${g}</button>`).join("");
}

function renderMarketCap() {
  renderCapFilter();
  const tbody = $("#cap-table tbody");
  const cards = sorted(CAP.cards.filter((c) => !capGames.size || capGames.has(c.game)), SORT.cap);
  $("#cap-note").textContent = CAP.note;
  tbody.innerHTML = cards.map((c) => `
    <tr onclick="openCard()">
      <td class="num rank">${c.rank}</td>
      <td><div class="row-name"><img class="row-thumb" src="${c.image}" alt="${c.name}">
        <div><div class="row-title">${c.name} <span class="grade-chip">PSA 10</span></div><div class="row-sub">${c.game} · ${c.set}${String(c.set).includes(c.year) ? "" : " · " + c.year}</div></div></div></td>
      <td class="num strong">$${c.cap}M</td>${pct(c.d7)}${pct(c.d30)}${pct(c.d90)}
      <td class="num">$${fmt(c.entry)}</td><td class="num">${c.momentum}</td><td class="num">${c.dominance.toFixed(2)}%</td><td class="num">${fmt(c.pop)}</td>
      <td class="num">${spark(c.spark, c.d90 >= 0)}</td>
    </tr>`).join("");
  markSort($("#cap-table"), SORT.cap);
}

$("#cap-filter").addEventListener("click", (e) => {
  const b = e.target.closest("button"); if (!b) return;
  const g = b.dataset.game;
  if (g === "all") capGames.clear(); else if (capGames.has(g)) capGames.delete(g); else capGames.add(g);
  renderMarketCap();
});
bindSort($("#ov-table"), "ov", renderOverview);
bindSort($("#cap-table"), "cap", renderMarketCap);

import "./style.css";
import { api, readQrFromClipboard, type ServerProfile, type Subscription, type Settings, type DnsResult, type IpResult, type ReqInfo, type AppUpdateInfo } from "./api";
import { t, setLang, getLang, isRTL, type Lang } from "./i18n";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

const $app = document.getElementById("app")!;

function el(tag: string, cls?: string, text?: string): any {
  const e = document.createElement(tag) as any;
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function icon(name: string): HTMLElement {
  const span = document.createElement("span");
  span.className = "ph";
  const icons: Record<string, string> = {
    servers: '<path d="M4 6h16M4 12h16M4 18h10" stroke-width="2.4" stroke-linecap="round"/>',
    routing: '<path d="M4 16c4 0 4-8 8-8s4 8 8 8" fill="none" stroke-width="2.4" stroke-linecap="round"/>',
    subs: '<path d="M5 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v12l-7-3.5L5 19V7z" fill="none" stroke-width="2.4" stroke-linejoin="round"/>',
    settings: '<circle cx="12" cy="12" r="3.2" fill="none" stroke-width="2.4"/><path d="M12 3v2.6M12 18.4V21M3 12h2.6M18.4 12H21M5.6 5.6l1.9 1.9M16.5 16.5l1.9 1.9M18.4 5.6l-1.9 1.9M7.5 16.5l-1.9 1.9" stroke-linecap="round" stroke-width="2.4"/>',
    logs: '<path d="M6 4h12v16H6zM9 8h6M9 12h6M9 16h4" fill="none" stroke-width="2.2"/>',
    about: '<circle cx="12" cy="12" r="8.5" fill="none" stroke-width="2.4"/><path d="M12 11v5M12 8v.2" stroke-linecap="round" stroke-width="2.6"/>',
    power: '<path d="M12 3v8M6.5 7a8 8 0 1 0 11 0" fill="none" stroke-width="2.6" stroke-linecap="round"/>',
    plus: '<path d="M12 5v14M5 12h14" stroke-width="2.6" stroke-linecap="round"/>',
    apps: '<rect x="4" y="4" width="7" height="7" rx="1.5" fill="none" stroke-width="2.2"/><rect x="13" y="4" width="7" height="7" rx="1.5" fill="none" stroke-width="2.2"/><rect x="4" y="13" width="7" height="7" rx="1.5" fill="none" stroke-width="2.2"/><path d="M16.5 15.5h3v3h-3z" fill="none" stroke-width="2.2"/>',
    qr: '<rect x="4" y="4" width="6" height="6" rx="1" fill="none" stroke-width="2.2"/><rect x="14" y="4" width="6" height="6" rx="1" fill="none" stroke-width="2.2"/><rect x="4" y="14" width="6" height="6" rx="1" fill="none" stroke-width="2.2"/><path d="M14 14h3M17 17h3M14 20h3" stroke-width="2.2" stroke-linecap="round"/>',
    scan: '<path d="M4 8V5a1 1 0 0 1 1-1h3M20 8V5a1 1 0 0 0-1-1h-3M4 16v3a1 1 0 0 0 1 1h3M20 16v3a1 1 0 0 1-1 1h-3M3 12h18" fill="none" stroke-width="2.2" stroke-linecap="round"/>',
    check: '<path d="M5 13l4 4L19 7" fill="none" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>',
    refresh: '<path d="M20 12a8 8 0 1 1-2.3-5.7M20 3v4h-4" fill="none" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>',
    trash: '<path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v6M14 11v6" fill="none" stroke-width="2.2" stroke-linecap="round"/>',
    pencil: '<path d="M4 20l4-1L19 8l-3-3L5 16l-1 4zM13 6l3 3" fill="none" stroke-width="2.2" stroke-linejoin="round"/>',
    copy: '<path d="M8 8V4h12v12h-4M4 8h12v12H4z" fill="none" stroke-width="2.2" stroke-linejoin="round"/>',
    globe: '<circle cx="12" cy="12" r="8.5" fill="none" stroke-width="2.2"/><path d="M3.5 12h17M12 3.5c3 3 3 14 0 17-3-3-3-14 0-17z" fill="none" stroke-width="2.2"/>',
    shield: '<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z" fill="none" stroke-width="2.2" stroke-linejoin="round"/>',
    pin: '<path d="M12 21s-6-6.5-6-11a6 6 0 1 1 12 0c0 4.5-6 11-6 11z" fill="none" stroke-width="2.2"/><circle cx="12" cy="10" r="2.4" fill="none" stroke-width="2.2"/>',
    dl: '<path d="M12 4v11M7 11l5 5 5-5M5 20h14" fill="none" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>',
  };
  span.innerHTML = `<svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none">${icons[name] || ""}</svg>`;
  return span;
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

let settings: Settings | null = null;
let servers: ServerProfile[] = [];
let subs: Subscription[] = [];

// startup release check (same pattern as the requirements card)
let updateInfo: AppUpdateInfo | null = null;
let updateDismissed = false;

async function refreshData() {
  settings = await api.getSettings();
  servers = await api.listServers();
  subs = await api.listSubscriptions();
}

// ---------------------------------------------------------------------------
// theme (light / dark / auto)
// ---------------------------------------------------------------------------

let themeMouseListener: ((e: MediaQueryListEvent) => void) | null = null;

function applyTheme(theme: string | undefined) {
  const root = document.documentElement;
  const pref = window.matchMedia("(prefers-color-scheme: dark)");
  const setClass = (dark: boolean) => root.classList.toggle("dark", dark);
  if (themeMouseListener) {
    pref.removeEventListener("change", themeMouseListener);
    themeMouseListener = null;
  }
  if (theme === "dark") {
    setClass(true);
  } else if (theme === "light") {
    setClass(false);
  } else {
    // auto: follow the OS color scheme
    setClass(pref.matches);
    themeMouseListener = (e) => setClass(e.matches);
    pref.addEventListener("change", themeMouseListener);
  }
}

// ---------------------------------------------------------------------------
// layout
// ---------------------------------------------------------------------------

let navEl: HTMLElement;
let mainEl: HTMLElement;
let statusDot: HTMLElement;
let statusText: HTMLElement;
let trafficEl: HTMLElement;
let currentView = "servers";

const NAV = [
  { id: "servers", icon: "servers", label: "nav.servers" },
  { id: "subs", icon: "subs", label: "nav.subs" },
  { id: "routing", icon: "routing", label: "nav.routing" },
  { id: "apps", icon: "apps", label: "nav.apps" },
  { id: "assets", icon: "dl", label: "nav.assets" },
  { id: "psiphon", icon: "shield", label: "nav.psiphon" },
  { id: "tools", icon: "globe", label: "nav.tools" },
  { id: "settings", icon: "settings", label: "nav.settings" },
  { id: "logs", icon: "logs", label: "nav.logs" },
  { id: "about", icon: "about", label: "nav.about" },
  { id: "connection", icon: "power", label: "nav.connection" },
];

function buildLayout() {
  $app.innerHTML = "";
  document.documentElement.setAttribute("dir", isRTL() ? "rtl" : "ltr");
  document.documentElement.setAttribute("lang", getLang());

  const shell = el("div", "shell");
  const sidebar = el("aside", "sidebar");

  const brand = el("div", "brand");
  const logo = el("div", "logo", "M");
  const brandText = el("div", "brand-text");
  brandText.append(el("div", "brand-name", t("appName")), el("div", "brand-sub", t("subtitle")));
  brand.append(logo, brandText);
  sidebar.append(brand);

  navEl = el("nav", "nav");
  for (const n of NAV) {
    const item = el("button", "nav-item");
    item.append(icon(n.icon), el("span", "", t(n.label)));
    item.dataset.view = n.id;
    item.addEventListener("click", () => switchView(n.id));
    navEl.append(item);
  }
  sidebar.append(navEl);

  // connect card at bottom
  const connectCard = el("div", "connect-card");
  const row = el("div", "conn-row");
  statusDot = el("span", "dot off");
  statusText = el("span", "status-label", t("status.disconnected"));
  row.append(statusDot, statusText);
  const btnRow = el("div", "conn-btns");
  const btnSystem = el("button", "btn primary", t("settings.systemMode"));
  btnSystem.onclick = () => doConnect("system");
  const btnTun = el("button", "btn", t("status.tunMode"));
  btnTun.onclick = () => doConnect("tun");
  const btnPsiphon = el("button", "btn", t("status.psiphonOnly"));
  btnPsiphon.onclick = () => doConnect("psiphon-only");
  const btnStop = el("button", "btn ghost", "Stop");
  btnStop.id = "btn-stop";
  btnStop.style.display = "none";
  btnStop.onclick = () => doDisconnect();
  btnRow.append(btnSystem, btnTun, btnPsiphon, btnStop);
  trafficEl = el("div", "traffic", "");
  connectCard.append(row, trafficEl, btnRow);
  sidebar.append(connectCard);

  mainEl = el("main", "main");
  shell.append(sidebar, mainEl);
  $app.append(shell);
  markNav();
}

function markNav() {
  navEl.querySelectorAll(".nav-item").forEach((n) => {
    (n as HTMLElement).classList.toggle("on", (n as HTMLElement).dataset.view === currentView);
  });
}

function switchView(v: string) {
  currentView = v;
  markNav();
  renderView();
}

// ---------------------------------------------------------------------------
// new-release banner (startup check, requirements-card pattern)
// ---------------------------------------------------------------------------

function openExternal(url: string) {
  // @ts-ignore injected by withGlobalTauri
  window.__TAURI__?.opener?.openUrl?.(url);
  if (!(window as any).__TAURI__) window.open(url, "_blank");
}

function buildUpdateBanner(): HTMLElement {
  const info = updateInfo!;
  const card = el("div", "card update-banner");
  card.append(
    el("h2", "", t("update.title")),
    el("p", "", `${t("update.current")}: ${info.current_version}  ->  ${t("update.latest")}: ${info.latest_version}`)
  );
  const actions = el("div", "actions");
  if (info.download_url) {
    const dl = el("button", "btn primary", t("update.download"));
    dl.onclick = () => openExternal(info.download_url);
    actions.append(dl);
  }
  const page = el("button", "btn", t("update.release"));
  page.onclick = () => openExternal(info.release_url);
  actions.append(page);
  const later = el("button", "btn ghost", t("update.later"));
  later.onclick = () => {
    updateDismissed = true;
    card.remove();
  };
  actions.append(later);
  card.append(actions);
  return card;
}

async function checkAppUpdateOnBoot() {
  try {
    const info = await api.appUpdateCheck();
    if (info.update_available && !updateDismissed) {
      updateInfo = info;
      if (mainEl) renderView();
    } else {
      updateInfo = info.update_available ? info : null;
    }
  } catch {
    // offline / rate-limited: stay silent
  }
}

function renderView() {
  mainEl.innerHTML = "";
  if (updateInfo?.update_available && !updateDismissed) {
    mainEl.append(buildUpdateBanner());
  }
  switch (currentView) {
    case "servers": renderServers(); break;
    case "subs": renderSubs(); break;
    case "routing": renderRouting(); break;
    case "apps": renderApps(); break;
    case "assets": renderAssets(); break;
    case "psiphon": renderPsiphon(); break;
    case "tools": renderTools(); break;
    case "settings": renderSettings(); break;
    case "logs": renderLogs(); break;
    case "about": renderAbout(); break;
    case "connection": renderConnect(); break;
  }
}

async function updateStatus() {
  try {
    const s = await api.status();
    const running = s.running;
    statusDot.className = `dot ${running ? "on" : "off"}`;
    statusText.textContent = running
      ? `${t(
s.mode === "tun"
        ? "status.tunMode"
        : s.mode === "psiphon-only"
          ? "status.psiphonOnly"
          : s.mode === "psiphon-after"
            ? "status.psiphonAfter"
            : "status.systemMode"
      )} - ${t("status.connected")}`
      : t("status.disconnected");
    const stop = document.getElementById("btn-stop");
    if (stop) stop.style.display = running ? "" : "none";
  } catch { /* window startup */ }
}

// ---------------------------------------------------------------------------
// servers view
// ---------------------------------------------------------------------------

function delayBadge(d: number | null): HTMLElement {
  const b = el("span", "badge");
  if (d == null) {
    b.textContent = "-";
  } else if (d <= 0) {
    b.className = "badge fail";
    b.textContent = t("logs.empty") === "" ? "x" : "timeout";
  } else if (d < 200) {
    b.className = "badge ok";
    b.textContent = `${d} ms`;
  } else if (d < 600) {
    b.className = "badge mid";
    b.textContent = `${d} ms`;
  } else {
    b.className = "badge fail";
    b.textContent = `${d} ms`;
  }
  return b;
}

function renderServers() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("servers.title")));
  const actions = el("div", "actions");
  const btnImport = el("button", "btn primary", t("servers.import"));
  btnImport.prepend(icon("plus"));
  btnImport.onclick = () => importDialog();
  const btnPing = el("button", "btn", t("servers.testAll"));
  btnPing.prepend(icon("refresh"));
  btnPing.onclick = async () => {
    btnPing.disabled = true;
    await api.testAllDelays();
    servers = await api.listServers();
    renderView();
  };
  const btnReal = el("button", "btn", t("servers.realAll"));
  btnReal.prepend(icon("globe"));
  btnReal.onclick = async () => {
    btnReal.disabled = true;
    try {
      await api.testAllReal();
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    btnReal.disabled = false;
    servers = await api.listServers();
    renderView();
  };
  const btnValidate = el("button", "btn", t("servers.validate"));
  btnValidate.prepend(icon("check"));
  btnValidate.onclick = async () => {
    if (!settings?.active_server) return alert(t("servers.noServers"));
    btnValidate.disabled = true;
    try {
      await api.validateConfig();
      alert(t("servers.validOk"));
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    btnValidate.disabled = false;
  };
  actions.append(btnImport, btnPing, btnReal, btnValidate);
  head.append(actions);
  wrap.append(head);

  const activeId = settings?.active_server || "";
  if (servers.length === 0) {
    wrap.append(el("div", "empty", t("servers.noServers")));
    mainEl.append(wrap);
    return;
  }

  const list = el("div", "server-list");
  for (const p of servers) {
    const item = el("div", "server-row");
    if (p.id === activeId) item.classList.add("active");

    const info = el("div", "srv-info");
    const top = el("div", "srv-top");
    const name = el("span", "srv-name", p.remark || `${p.address}:${p.port}`);
    const proto = el("span", "chip", p.protocol);
    top.append(name, proto);
    if (p.id === activeId) top.append(el("span", "chip active-chip", t("servers.active")));
    if (p.sub_id) {
      const sub = subs.find((s) => s.id === p.sub_id);
      if (sub) top.append(el("span", "chip sub-chip", sub.name));
    }
    const meta = el("div", "srv-meta", `${p.address}:${p.port} - ${p.security || "no-tls"} / ${p.net}`);
    info.append(top, meta);

    const right = el("div", "srv-actions");
    right.append(delayBadge(p.delay));

    const btnPing = el("button", "btn-icon", "");
    btnPing.title = t("servers.ping");
    btnPing.append(icon("refresh"));
    btnPing.onclick = async () => {
      btnPing.disabled = true;
      await api.testDelay(p.id);
      servers = await api.listServers();
      renderView();
    };
    const btnReal = el("button", "btn-icon", "");
    btnReal.title = t("servers.realTest");
    btnReal.append(icon("globe"));
    btnReal.onclick = async () => {
      btnReal.disabled = true;
      try {
        await api.runRealTest(p.id);
        servers = await api.listServers();
        renderView();
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
        btnReal.disabled = false;
      }
    };
    const btnQr = el("button", "btn-icon", "");
    btnQr.title = t("servers.qr");
    btnQr.append(icon("qr"));
    btnQr.onclick = () => showQr(p);
    const btnCopy = el("button", "btn-icon", "");
    btnCopy.title = t("servers.copy");
    btnCopy.append(icon("copy"));
    btnCopy.onclick = async () => {
      const link = await api.shareLink(p.id);
      await navigator.clipboard.writeText(link);
    };
    const btnUse = el("button", "btn-icon", "");
    btnUse.title = t("servers.setActive");
    btnUse.append(icon("pin"));
    btnUse.onclick = async () => {
      await api.setActive(p.id);
      settings = await api.getSettings();
      renderView();
    };
    const btnEdit = el("button", "btn-icon", "");
    btnEdit.title = t("servers.edit");
    btnEdit.append(icon("pencil"));
    btnEdit.onclick = () => editDialog(p);
    const btnDel = el("button", "btn-icon danger", "");
    btnDel.title = t("servers.delete");
    btnDel.append(icon("trash"));
    btnDel.onclick = async () => {
      if (!confirm(t("servers.delete") + ": " + (p.remark || p.address))) return;
      await api.deleteServer(p.id);
      servers = await api.listServers();
      renderView();
    };
    right.append(btnPing, btnReal, btnQr, btnCopy, btnUse, btnEdit, btnDel);
    item.append(info, right);
    item.addEventListener("dblclick", async () => {
      await api.setActive(p.id);
      settings = await api.getSettings();
      await doConnect("system");
      renderView();
    });
    list.append(item);
  }
  wrap.append(list);
  mainEl.append(wrap);
}

async function showQr(p: ServerProfile) {
  const dlg = el("div", "modal-back");
  const box = el("div", "modal");
  const title = el("h2", "", t("servers.qr"));
  const img = el("img", "qr-img") as HTMLImageElement;
  try {
    const link = await api.shareLink(p.id);
    const QR = (await import("qrcode")).default;
    img.src = await QR.toDataURL(link, { width: 320, margin: 1 });
    img.alt = link;
    box.append(title, img, el("div", "srv-meta", link));
  } catch (e: any) {
    box.append(title, el("p", "muted", String(e)));
  }
  const row = el("div", "modal-actions");
  const close = el("button", "btn ghost", t("servers.cancel"));
  close.onclick = () => dlg.remove();
  row.append(close);
  box.append(row);
  dlg.append(box);
  document.body.append(dlg);
}

async function scanFromClipboard(): Promise<void> {
  try {
    const data = await readQrFromClipboard();
    if (!data) {
      alert(t("servers.scanNone"));
      return;
    }
    const n = await api.importLinks(data, "");
    servers = await api.listServers();
    renderView();
    alert(String(n));
  } catch (e: any) {
    alert(`${t("error")}: ${e}`);
  }
}

async function scanFromScreen(): Promise<void> {
  try {
    // Windows screen-clip overlay puts the cropped screenshot into the clipboard
    // @ts-ignore injected by withGlobalTauri
    await (await import("@tauri-apps/plugin-opener")).openUrl("ms-screenclip:");
    await new Promise((r) => setTimeout(r, 3500));
    await scanFromClipboard();
  } catch (e: any) {
    alert(`${t("error")}: ${e}`);
  }
}

function importDialog() {
  const dlg = el("div", "modal-back");
  const box = el("div", "modal");
  const title = el("h2", "", t("servers.import"));
  const ta = el("textarea", "modal-ta") as HTMLTextAreaElement;
  ta.placeholder = t("servers.dialogAdd");
  const row = el("div", "modal-actions");
  const ok = el("button", "btn primary", t("servers.importBtn"));
  const cancel = el("button", "btn ghost", t("servers.cancel"));
  row.append(cancel, ok);
  const scanRow = el("div", "modal-actions");
  const btnClip = el("button", "btn", t("servers.scanClip"));
  btnClip.prepend(icon("scan"));
  btnClip.onclick = async () => {
    dlg.remove();
    await scanFromClipboard();
  };
  const btnScreen = el("button", "btn", t("servers.scanScreen"));
  btnScreen.prepend(icon("scan"));
  btnScreen.onclick = async () => {
    dlg.remove();
    await scanFromScreen();
  };
  scanRow.append(btnClip, btnScreen);
  box.append(title, ta, scanRow, row);
  dlg.append(box);
  document.body.append(dlg);
  cancel.onclick = () => dlg.remove();
  ok.onclick = async () => {
    const text = (ta.value || "").trim();
    if (!text) return;
    try {
      const n = await api.importLinks(text, "");
      dlg.remove();
      servers = await api.listServers();
      renderView();
      title.textContent = `${n}`;
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  ta.focus();
}

function editDialog(p: ServerProfile) {
  const dlg = el("div", "modal-back");
  const box = el("div", "modal wide");
  const title = el("h2", "", `${t("servers.edit")} - ${p.protocol}`);
  const form = el("div", "form");

  type Field = { key: keyof ServerProfile; label: string; type?: string };
  const common: Field[] = [
    { key: "remark", label: t("servers.fields.remark") },
    { key: "address", label: t("servers.fields.address") },
    { key: "port", label: t("servers.fields.port"), type: "number" },
  ];
  const authField: Field[] =
    p.protocol === "shadowsocks"
      ? [
          { key: "method", label: t("servers.fields.method") },
          { key: "auth", label: t("servers.fields.auth") },
        ]
      : [{ key: "auth", label: t("servers.fields.auth") }];
  const net: Field[] = [
    { key: "net", label: t("servers.fields.net") },
    { key: "header_type", label: t("servers.fields.header") },
    { key: "host", label: t("servers.fields.host") },
    { key: "path", label: t("servers.fields.path") },
  ];
  const tls: Field[] = [
    { key: "security", label: t("servers.fields.security") },
    { key: "sni", label: t("servers.fields.sni") },
    { key: "fingerprint", label: t("servers.fields.fingerprint") },
    { key: "pbk", label: t("servers.fields.pbk") },
    { key: "sid", label: t("servers.fields.sid") },
    { key: "flow", label: t("servers.fields.flow") },
    { key: "alpn", label: t("servers.fields.alpn") },
  ];

  const inputs: Record<string, HTMLInputElement> = {};
  for (const group of [common, authField, net, tls]) {
    if (group === net && !["ws", "grpc", "tcp", "kcp", "h2", "httpupgrade"].includes(p.net)) continue;
    if (group === tls && (p.security === "" || p.security === "none")) continue;
    for (const f of group) {
      const field = el("div", "field");
      field.append(el("label", "", f.label));
      const input = el("input") as HTMLInputElement;
      input.type = f.type || "text";
      input.value = String((p as any)[f.key] ?? "");
      inputs[f.key] = input;
      field.append(input);
      form.append(field);
    }
  }

  const row = el("div", "modal-actions");
  const cancel = el("button", "btn ghost", t("servers.cancel"));
  const ok = el("button", "btn primary", t("servers.edit"));
  row.append(cancel, ok);
  box.append(title, form, row);
  dlg.append(box);
  document.body.append(dlg);
  cancel.onclick = () => dlg.remove();
  ok.onclick = async () => {
    for (const [k, v] of Object.entries(inputs)) {
      (p as any)[k] = v.type === "number" ? Number(v.value || 0) : v.value;
    }
    await api.saveServer(p);
    servers = await api.listServers();
    dlg.remove();
    renderView();
  };
}

// ---------------------------------------------------------------------------
// subscriptions view
// ---------------------------------------------------------------------------

function renderSubs() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("subs.title")));
  const actions = el("div", "actions");
  const btnAdd = el("button", "btn primary", t("subs.add"));
  btnAdd.prepend(icon("plus"));
  btnAdd.onclick = () => subDialog(null);
  actions.append(btnAdd);
  // Get link button to fetch configs from subscription URL
  const btnGetLink = el("button", "btn", t("subs.getLink"));
  btnGetLink.prepend(icon("dl"));
  btnGetLink.onclick = () => getLinkDialog();
  actions.append(btnGetLink);
  // Import from clipboard button
  const btnImportClip = el("button", "btn", t("servers.scanClip"));
  btnImportClip.prepend(icon("scan"));
  btnImportClip.onclick = async () => {
    try {
      const data = await readQrFromClipboard();
      if (!data) {
        alert(t("servers.scanNone"));
        return;
      }
      const n = await api.importLinks(data, "");
      await refreshData();
      renderView();
      alert(String(n));
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  actions.append(btnImportClip);
  head.append(actions);
  wrap.append(head);

  if (subs.length === 0) {
    wrap.append(el("div", "empty", t("subs.none")));
    mainEl.append(wrap);
    return;
  }

  const list = el("div", "server-list");
  for (const s of subs) {
    const item = el("div", "server-row");
    const info = el("div", "srv-info");
    const top = el("div", "srv-top");
    top.append(el("span", "srv-name", s.name));
    info.append(top, el("div", "srv-meta", s.url));
    const right = el("div", "srv-actions");
    const count = servers.filter((p) => p.sub_id === s.id).length;
    right.append(el("span", "badge", `${count} servers`));
    const btnUpd = el("button", "btn-icon");
    btnUpd.title = t("subs.updateNow");
    btnUpd.append(icon("refresh"));
    btnUpd.onclick = async () => {
      btnUpd.disabled = true;
      try {
        await api.updateSubscription(s.id);
        await refreshData();
        renderView();
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
        btnUpd.disabled = false;
      }
    };
    const btnEdit = el("button", "btn-icon");
    btnEdit.title = t("servers.edit");
    btnEdit.append(icon("pencil"));
    btnEdit.onclick = () => subDialog(s);
    const btnDel = el("button", "btn-icon danger");
    btnDel.title = t("servers.delete");
    btnDel.append(icon("trash"));
    btnDel.onclick = async () => {
      if (!confirm(t("subs.confirmDelete"))) return;
      await api.deleteSubscription(s.id);
      await refreshData();
      renderView();
    };
    right.append(btnUpd, btnEdit, btnDel);
    item.append(info, right);
    list.append(item);
  }
  wrap.append(list);
  mainEl.append(wrap);
}

function subDialog(s: Subscription | null) {
  const dlg = el("div", "modal-back");
  const box = el("div", "modal");
  const title = el("h2", "", s ? t("servers.edit") : t("subs.add"));
  const form = el("div", "form");
  const mk = (label: string, val: string, type = "text"): HTMLInputElement => {
    const f = el("div", "field");
    f.append(el("label", "", label));
    const i = el("input") as HTMLInputElement;
    i.type = type;
    i.value = val;
    f.append(i);
    form.append(f);
    return i;
  };
  const inName = mk(t("subs.name"), s?.name || "");
  const inUrl = mk(t("subs.url"), s?.url || "");
  const inHours = mk(t("subs.interval"), String(s?.update_interval_hours || 6), "number");
  const row = el("div", "modal-actions");
  const cancel = el("button", "btn ghost", t("servers.cancel"));
  const ok = el("button", "btn primary", s ? t("servers.edit") : t("subs.add"));
  row.append(cancel, ok);
  box.append(title, form, row);
  dlg.append(box);
  document.body.append(dlg);
  cancel.onclick = () => dlg.remove();
  ok.onclick = async () => {
    try {
      if (s) {
        s.name = inName.value || s.name;
        s.url = inUrl.value || s.url;
        s.update_interval_hours = Number(inHours.value || 6);
        await api.editSubscription(s);
      } else {
        await api.addSubscription(
          inName.value || "sub",
          inUrl.value.trim(),
          Number(inHours.value || 6),
          true
        );
      }
      dlg.remove();
      await refreshData();
      renderView();
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  inName.focus();
}

// ---------------------------------------------------------------------------
// get link dialog for fetching subscription configs
// ---------------------------------------------------------------------------

async function getLinkDialog() {
  const dlg = el("div", "modal-back");
  const box = el("div", "modal");
  const title = el("h2", "", t("subs.getLink"));
  const form = el("div", "form");
  const mk = (label: string, val: string, type = "text"): HTMLInputElement => {
    const f = el("div", "field");
    f.append(el("label", "", label));
    const i = el("input") as HTMLInputElement;
    i.type = type;
    i.value = val;
    f.append(i);
    form.append(f);
    return i;
  };
  const inUrl = mk(t("subs.url"), "");
  inUrl.focus();
  const row = el("div", "modal-actions");
  const cancel = el("button", "btn ghost", t("servers.cancel"));
  const ok = el("button", "btn primary", t("subs.getLink"));
  row.append(cancel, ok);
  box.append(title, form, row);
  dlg.append(box);
  document.body.append(dlg);
  cancel.onclick = () => dlg.remove();
  ok.onclick = async () => {
    try {
      const url = inUrl.value.trim();
      if (!url) return;
      // Show fetching state
      ok.disabled = true;
      ok.textContent = t("requirements.updating");
      const n = await api.importLinks(url, "");
      dlg.remove();
      await refreshData();
      renderView();
      alert(`${t("subs.updated")}: ${n}`);
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    ok.disabled = false;
    ok.textContent = t("subs.getLink");
  };
}

// ---------------------------------------------------------------------------
// routing view
// ---------------------------------------------------------------------------

function renderRouting() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("routing.title")));
  wrap.append(head);
  if (!settings) return;
  const s = settings;

  const card1 = el("div", "card");
  card1.append(el("h2", "", t("routing.preset")));
  const selWrap = el("div", "field");
  const preset = el("select") as HTMLSelectElement;
  const presetOptions: [string, string][] = [
    ["global", t("routing.presets.global")],
    ["ir", t("routing.presets.ir")],
    ["ir_cn", t("routing.presets.ir_cn")],
  ];
  for (const [k, v] of presetOptions) {
    const o = el("option", "", v) as HTMLOptionElement;
    o.value = k;
    preset.append(o);
  }
  preset.value = s.routing_preset;
  preset.onchange = () => save({ routing_preset: preset.value });
  selWrap.append(preset);
  card1.append(selWrap);
  wrap.append(card1);

  const mkList = (title: string, key: keyof Settings) => {
    const card = el("div", "card");
    card.append(el("h2", "", title));
    const ta = el("textarea", "lines") as HTMLTextAreaElement;
    ta.placeholder = "example.com";
    ta.value = String(s[key] ?? "");
    ta.addEventListener("change", () => save({ [key]: ta.value }));
    card.append(ta);
    wrap.append(card);
  };
  mkList(t("routing.customProxy"), "custom_proxy");
  mkList(t("routing.customDirect"), "custom_direct");
  mkList(t("routing.customBlock"), "custom_block");

  // geo status
  const geoCard = el("div", "card");
  geoCard.append(el("h2", "", t("routing.geoTitle")));
  const geoRow = el("div", "actions");
  const geoStatus = el("span", "badge");
  api.geoExists().then((ok) => {
    geoStatus.className = `badge ${ok ? "ok" : ""}`;
    geoStatus.textContent = ok ? t("routing.geoOk") : t("routing.geoMissing");
  });
  const btnGeo = el("button", "btn", t("routing.geoDownload"));
  btnGeo.prepend(icon("dl"));
  btnGeo.onclick = async () => {
    btnGeo.disabled = true;
    btnGeo.textContent = "...";
    try {
      await api.downloadGeo();
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    renderView();
  };
  geoRow.append(geoStatus, btnGeo);
  geoCard.append(geoRow);
  geoCard.append(el("p", "muted", t("routing.needReconnect")));
  wrap.append(geoCard);

  mainEl.append(wrap);
}

async function save(patch: Partial<Settings>) {
  if (!settings) return;
  settings = { ...settings, ...patch };
  await api.setSettings(settings);
}

// ---------------------------------------------------------------------------
// requirements card (check + upgrade engines)
// ---------------------------------------------------------------------------

async function renderRequirementsCard(wrap: HTMLElement) {
  const card = el("div", "card");
  card.append(el("h2", "", t("requirements.title")));
  card.append(el("p", "muted", t("requirements.note")));
  const checkRow = el("div", "actions");
  const checkBtn = el("button", "btn primary", t("requirements.check"));
  checkRow.append(checkBtn);
  card.append(checkRow);
  const list = el("div", "lines") as HTMLElement;
  card.append(list);
  wrap.append(card);

  const paint = (items: ReqInfo[]) => {
    list.innerHTML = "";
    for (const it of items) {
      const row = el("div", "switch-row");
      row.append(el("span", "", it.title));
      const right = el("div", "srv-actions");
      right.append(
        el("span", `badge ${it.exists ? "ok" : "fail"}`, it.exists ? t("requirements.ok") : t("requirements.missing"))
      );
      if (it.exists && it.installed_version) {
        right.append(el("span", "badge", `${t("requirements.version")} ${it.installed_version.slice(0, 40)}`));
      }
      if (it.available_version && it.update_available) {
        right.append(el("span", "badge mid", t("requirements.updateFound")));
      }
      if (it.exists) {
        right.append(el("span", "badge", fmtBytes(it.size)));
      }
      const btn = el("button", "btn", it.exists ? t("requirements.upgrade") : t("requirements.upgrade"));
      btn.onclick = async () => {
        btn.disabled = true;
        btn.textContent = t("requirements.updating");
        try {
          await api.requirementsUpgrade(it.key);
          paint(await api.requirementsStatus());
        } catch (e: any) {
          alert(`${t("error")}: ${e}`);
          btn.disabled = false;
          btn.textContent = t("requirements.upgrade");
        }
      };
      right.append(btn);
      row.append(right);
      list.append(row);
    }
    checkBtn.disabled = false;
    checkBtn.textContent = t("requirements.check");
  };

  const runCheck = async () => {
    checkBtn.disabled = true;
    checkBtn.textContent = t("requirements.checking");
    try {
      paint(await api.requirementsStatus());
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
      checkBtn.disabled = false;
      checkBtn.textContent = t("requirements.check");
    }
  };
  checkBtn.onclick = () => runCheck();
  // initial status fetch (local presence + remote versions)
  runCheck();
}

// ---------------------------------------------------------------------------
// per-app proxy view (effective in TUN mode)
// ---------------------------------------------------------------------------

let procList: string[] | null = null;

function renderApps() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("apps.title")));
  wrap.append(head);
  if (!settings) return;
  const note = el("p", "muted", t("apps.note"));
  wrap.append(note);

  const modeCard = el("div", "card");
  modeCard.append(el("h2", "", t("apps.mode")));
  const f = el("div", "field");
  const sel = el("select") as HTMLSelectElement;
  for (const [k, v] of [
    ["off", t("apps.off")],
    ["bypass", t("apps.bypass")],
    ["allow", t("apps.allow")],
  ] as [string, string][]) {
    const o = el("option", "", v) as HTMLOptionElement;
    o.value = k;
    sel.append(o);
  }
  sel.value = settings.per_app_mode || "off";
  sel.onchange = () => save({ per_app_mode: sel.value });
  f.append(sel);
  modeCard.append(f);
  wrap.append(modeCard);

  const listCard = el("div", "card");
  listCard.append(el("h2", "", t("apps.list")));
  const listWrap = el("div", "app-list");
  listWrap.append(el("p", "muted", t("apps.loading")));
  listCard.append(listWrap);
  wrap.append(listCard);
  mainEl.append(wrap);

  api.listProcesses().then((procs) => {
    procList = procs;
    listWrap.innerHTML = "";
    const chosen = new Set(
      (settings?.per_app_list || "")
        .split(/\r?\n/)
        .map((l: string) => l.trim())
        .filter(Boolean)
    );
    const search = el("input") as HTMLInputElement;
    search.placeholder = t("apps.search");
    search.className = "app-search";
    const listEl = el("div", "app-items");
    const renderItems = () => {
      listEl.innerHTML = "";
      const q = (search.value || "").toLowerCase();
      for (const pname of procs) {
        if (q && !pname.toLowerCase().includes(q)) continue;
        const row = el("div", "switch-row");
        row.append(el("span", "mono", pname));
        const sw = el("button", `switch ${chosen.has(pname) ? "on" : ""}`);
        sw.onclick = async () => {
          if (chosen.has(pname)) chosen.delete(pname);
          else chosen.add(pname);
          sw.className = `switch ${chosen.has(pname) ? "on" : ""}`;
          await save({ per_app_list: [...chosen].sort().join("\n") });
        };
        row.append(sw);
        listEl.append(row);
      }
    };
    search.addEventListener("input", renderItems);
    renderItems();
    listWrap.innerHTML = "";
    listWrap.append(search, listEl);
  });
}

// ---------------------------------------------------------------------------
// geo assets view
// ---------------------------------------------------------------------------

function renderAssets() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("assets.title")));
  wrap.append(head);
  wrap.append(el("p", "muted", t("assets.note")));

  const card = el("div", "card");
  api.assetsInfo().then((info) => {
    for (const a of info) {
      const row = el("div", "switch-row");
      const label = el("span", "mono", a.name);
      row.append(label);
      const right = el("div", "srv-actions");
      right.append(
        el("span", "badge", a.exists ? fmtBytes(a.size) : t("assets.missing"))
      );
      const btnDel = el("button", "btn-icon danger");
      btnDel.append(icon("trash"));
      btnDel.title = t("servers.delete");
      btnDel.disabled = !a.exists;
      btnDel.onclick = async () => {
        await api.deleteGeo(a.name);
        renderView();
      };
      right.append(btnDel);
      row.append(right);
      card.append(row);
    }
    wrap.append(card);
    const actions = el("div", "actions");
    const btnDl = el("button", "btn primary", t("routing.geoDownload"));
    btnDl.prepend(icon("dl"));
    btnDl.onclick = async () => {
      btnDl.disabled = true;
      btnDl.textContent = "...";
      try {
        await api.downloadGeo();
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
      }
      renderView();
    };
    actions.append(btnDl);
    wrap.append(actions);
  });
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// psiphon view
// ---------------------------------------------------------------------------

const PSIPHON_REGIONS: [string, string][] = [
  ["", "psiphon.regionAuto"],
  ["US", "United States"],
  ["CA", "Canada"],
  ["GB", "United Kingdom"],
  ["NL", "Netherlands"],
  ["DE", "Germany"],
  ["SE", "Sweden"],
  ["ES", "Spain"],
  ["SG", "Singapore"],
  ["AU", "Australia"],
  ["HK", "Hong Kong"],
  ["TR", "Turkey"],
];

function renderPsiphon() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("psiphon.title")));
  const actions = el("div", "actions");
  const statusBadge = el("span", "badge");
  api.status().then((s) => {
    const run = s.running && (s.mode === "psiphon-only" || s.mode === "psiphon-after");
    statusBadge.className = `badge ${run ? "ok" : ""}`;
    statusBadge.textContent = run ? t("status.connected") : t("status.disconnected");
  });
  actions.append(statusBadge);
  head.append(actions);
  wrap.append(head);
  wrap.append(el("p", "muted", t("psiphon.note")));
  if (!settings) return;
  const s = settings;

  // connect cards
  const grid = el("div", "bento");
  for (const cfg of [
    { mode: "psiphon-only", title: t("psiphon.only"), desc: t("psiphon.onlyDesc") },
    { mode: "psiphon-after", title: t("psiphon.after"), desc: t("psiphon.afterDesc") },
  ]) {
    const box = el("div", "bento-box");
    box.append(el("h3", "", cfg.title), el("p", "muted", cfg.desc));
    const row = el("div", "actions");
    const btn = el("button", "btn primary", t("psiphon.connect"));
    btn.onclick = async () => {
      await save({ psiphon_mode: cfg.mode === "psiphon-only" ? "only" : "after" });
      btn.disabled = true;
      btn.textContent = "...";
      try {
        await api.connect(cfg.mode);
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
      }
      updateStatus();
      renderView();
    };
    row.append(btn);
    box.append(row);
    grid.append(box);
  }
  // stop button
  const stopBox = el("div", "bento-box");
  stopBox.append(el("h3", "", t("psiphon.stop")));
  const stopRow = el("div", "actions");
  const btnStop = el("button", "btn ghost", t("psiphon.stop"));
  btnStop.onclick = async () => {
    await api.disconnect();
    updateStatus();
    renderView();
  };
  stopRow.append(btnStop);
  stopBox.append(stopRow);
  grid.append(stopBox);
  wrap.append(grid);

  // settings card
  const card = el("div", "card");
  card.append(el("h2", "", t("settings.psiphonCard")));
  const grid2 = el("div", "grid2");
  const mkNum = (title: string, key: keyof Settings) => {
    const f = el("div", "field");
    f.append(el("label", "", title));
    const i = el("input") as HTMLInputElement;
    i.type = "number";
    i.value = String((s as any)[key] ?? "");
    i.addEventListener("change", () => save({ [key]: Number(i.value || 0) } as any));
    f.append(i);
    return f;
  };
  const mkSel = (title: string, key: keyof Settings, options: [string, string][]) => {
    const f = el("div", "field");
    f.append(el("label", "", title));
    const sel = el("select") as HTMLSelectElement;
    for (const [k, v] of options) {
      const o = el("option", "", v) as HTMLOptionElement;
      o.value = k;
      sel.append(o);
    }
    sel.value = String((s as any)[key] ?? "");
    sel.onchange = () => save({ [key]: sel.value } as any);
    f.append(sel);
    return f;
  };
  const mkSw = (title: string, key: keyof Settings) => {
    const row = el("div", "switch-row");
    row.append(el("span", "", title));
    const sw = el("button", `switch ${(s as any)[key] ? "on" : ""}`);
    sw.onclick = async () => {
      await save({ [key]: !(s as any)[key] } as any);
      (s as any)[key] = !(s as any)[key];
      sw.className = `switch ${(s as any)[key] ? "on" : ""}`;
    };
    row.append(sw);
    return row;
  };
  grid2.append(
    mkNum(t("psiphon.socksPort"), "psiphon_socks_port"),
    mkNum(t("psiphon.httpPort"), "psiphon_http_port"),
    mkSel(t("psiphon.region"), "psiphon_region", PSIPHON_REGIONS.map(([k, v]) => [k, k ? v : t(v)] as [string, string])),
    mkSel(t("psiphon.protocol"), "psiphon_protocol", [
      ["auto", t("psiphon.protoAuto")],
      ["normal", t("psiphon.protoNormal")],
      ["conduit", t("psiphon.protoConduit")],
    ])
  );
  card.append(grid2);
  card.append(mkSw(t("psiphon.aggressive"), "psiphon_aggressive"));
  wrap.append(card);
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// connection view (separate tab: connect button + method options)
// ---------------------------------------------------------------------------

function renderConnect() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("connection.title")));
  wrap.append(head);

  if (!settings) return;

  // connect status row (reflects the live runtime state, not stale settings)
  const statusRow = el("div", "conn-row");
  const statusDot = el("span", "dot off");
  const statusLabel = el("span", "status-label", t("status.disconnected"));
  statusRow.append(statusDot, statusLabel);
  wrap.append(statusRow);
  const modeLabel: Record<string, string> = {
    system: "status.systemMode",
    tun: "status.tunMode",
    "psiphon-only": "status.psiphonOnly",
    "psiphon-after": "status.psiphonAfter",
  };

  // method selection grid
  const grid = el("div", "bento");
  const sysBox = el("div", "bento-box");
  sysBox.append(
    el("h3", "", t("settings.systemMode")),
    el("p", "muted", t("settings.systemModeDesc")),
    el("button", "btn primary", t("connection.connect"))
  );
  const btnSystem = sysBox.querySelector("button.btn.primary");
  btnSystem.onclick = async () => {
    await doConnect("system");
    updateStatus();
    renderView();
  };

  const tunBox = el("div", "bento-box");
  tunBox.append(
    el("h3", "", t("settings.tunMode")),
    el("p", "muted", t("settings.tunModeDesc")),
    el("button", "btn", t("connection.connect"))
  );
  const btnTun = tunBox.querySelector("button.btn");
  btnTun.onclick = async () => {
    await doConnect("tun");
    updateStatus();
    renderView();
  };

  const psiphonBox = el("div", "bento-box");
  psiphonBox.append(
    el("h3", "", t("status.psiphonOnly")),
    el("p", "muted", t("psiphon.onlyDesc")),
    el("button", "btn", t("connection.connect"))
  );
  const btnPsiphonOnly = psiphonBox.querySelector("button.btn");
  btnPsiphonOnly.onclick = async () => {
    await doConnect("psiphon-only");
    updateStatus();
    renderView();
  };

  const psiphonAfterBox = el("div", "bento-box");
  psiphonAfterBox.append(
    el("h3", "", t("status.psiphonAfter")),
    el("p", "muted", t("psiphon.afterDesc")),
    el("button", "btn", t("connection.connect"))
  );
  const btnPsiphonAfter = psiphonAfterBox.querySelector("button.btn");
  btnPsiphonAfter.onclick = async () => {
    await doConnect("psiphon-after");
    updateStatus();
    renderView();
  };
  grid.append(sysBox, tunBox, psiphonBox, psiphonAfterBox);
  wrap.append(grid);

  // highlight the box matching the live connection state
  const boxes: [HTMLElement, string][] = [
    [sysBox, "system"],
    [tunBox, "tun"],
    [psiphonBox, "psiphon-only"],
    [psiphonAfterBox, "psiphon-after"],
  ];
  api.status().then((st) => {
    statusDot.className = `dot ${st.running ? "on" : "off"}`;
    statusLabel.textContent = st.running
      ? t(modeLabel[st.mode ?? ""] ?? "status.connected")
      : t("status.disconnected");
    for (const [box, mode] of boxes) {
      box.classList.toggle("selected", st.running && st.mode === mode);
    }
  });
}

  // ---------------------------------------------------------------------------
  // tools view (DNS scanner / IP scanner / network tools)
  // ---------------------------------------------------------------------------

function renderTools() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("tools.title")));
  wrap.append(head);
  if (!settings) return;

  // ---- DNS scanner ----
  const dnsCard = el("div", "card");
  dnsCard.append(el("h2", "", t("tools.dnsTitle")), el("p", "muted", t("tools.dnsNote")));
  const dnsRow = el("div", "grid2");
  const dnsField = el("div", "field");
  dnsField.append(el("label", "", t("tools.dnsDomain")));
  const dnsInput = el("input") as HTMLInputElement;
  dnsInput.value = "www.google.com";
  dnsField.append(dnsInput);
  const dnsBtnCol = el("div", "field");
  const dnsBtn = el("button", "btn primary", t("tools.dnsRun"));
  dnsBtnCol.append(dnsBtn);
  dnsRow.append(dnsField, dnsBtnCol);
  dnsCard.append(dnsRow);
  const dnsOut = el("div", "lines") as HTMLElement;
  dnsOut.style.display = "none";
  dnsCard.append(dnsOut);
  dnsBtn.onclick = async () => {
    const domain = dnsInput.value.trim();
    if (!domain) return;
    dnsBtn.disabled = true;
    dnsBtn.textContent = "...";
    try {
      const res: DnsResult[] = await api.dnsScan(domain);
      dnsOut.style.display = "";
      dnsOut.innerHTML = "";
      const statusLabel = (st: string) =>
        st === "ok"
          ? t("tools.dnsOk")
          : st === "timeout"
            ? t("tools.dnsTimeout")
            : st === "empty"
              ? t("tools.dnsEmpty")
              : st === "differs"
                ? t("tools.dnsDiffers")
                : st;
      for (const r of res) {
        const line = el("div", "switch-row");
        line.append(
          el("span", "mono", `${r.server} (${r.kind})`),
          el("span", "", statusLabel(r.status)),
          el("span", "badge", r.delay != null ? `${r.delay} ms` : "-"),
          el("span", "mono", r.ips.slice(0, 2).join(" "))
        );
        dnsOut.append(line);
      }
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    dnsBtn.disabled = false;
    dnsBtn.textContent = t("tools.dnsRun");
  };
  wrap.append(dnsCard);

  // ---- Mahsa IP scanner ----
  const ipCard = el("div", "card");
  ipCard.append(el("h2", "", t("tools.ipTitle")), el("p", "muted", t("tools.ipNote")));
  const ipGrid = el("div", "grid3");
  const mkField = (label: string) => {
    const f = el("div", "field");
    if (label) f.append(el("label", "", label));
    return f;
  };
  const presetF = mkField(t("tools.ipPreset"));
  const presetSel = el("select") as HTMLSelectElement;
  for (const [k, v] of [
    ["cloudflare", "Cloudflare"],
    ["google", "Google"],
    ["digitalocean", "DigitalOcean"],
    ["custom", "Custom"],
  ] as [string, string][]) {
    const o = el("option", "", v) as HTMLOptionElement;
    o.value = k;
    presetSel.append(o);
  }
  presetF.append(presetSel);
  const portF = mkField(t("tools.ipPort"));
  const portIn = el("input") as HTMLInputElement;
  portIn.type = "number";
  portIn.value = "443";
  portF.append(portIn);
  const btnF = mkField("");
  const ipBtn = el("button", "btn primary", t("tools.ipRun"));
  btnF.append(ipBtn);
  ipGrid.append(presetF, portF, btnF);
  ipCard.append(ipGrid);
  const customTa = el("textarea", "lines") as HTMLTextAreaElement;
  customTa.placeholder = "104.16.0.0/16\n172.64.0.0/16";
  customTa.style.display = "none";
  presetSel.onchange = () => {
    customTa.style.display = presetSel.value === "custom" ? "" : "none";
  };
  ipCard.append(customTa);
  const ipOut = el("div", "lines") as HTMLElement;
  ipCard.append(ipOut);
  ipBtn.onclick = async () => {
    ipBtn.disabled = true;
    ipBtn.textContent = t("tools.ipScanning");
    ipOut.innerHTML = "";
    try {
      const res: IpResult[] = await api.ipScan(
        presetSel.value,
        customTa.value,
        Number(portIn.value || 443)
      );
      const max = 24;
      let shown = 0;
      for (const r of res) {
        if (shown >= max) break;
        if (!r.ok) continue;
        shown++;
        const row = el("div", "switch-row");
        row.append(
          el("span", "mono", r.ip),
          el("span", "", t("tools.dnsOk")),
          el("span", "badge", `${r.delay} ms`)
        );
        const useBtn = el("button", "btn", t("tools.ipUse"));
        useBtn.onclick = async () => {
          await save({ cdn_ip: r.ip, cdn_mode: "manual" });
          useBtn.textContent = t("saved");
        };
        row.append(useBtn);
        ipOut.append(row);
      }
      if (shown === 0) {
        ipOut.append(el("p", "muted", `- ${t("tools.dnsTimeout")}`));
      }
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    ipBtn.disabled = false;
    ipBtn.textContent = t("tools.ipRun");
  };
  wrap.append(ipCard);

  // ---- Network tools ----
  const netCard = el("div", "card");
  netCard.append(el("h2", "", t("tools.netTitle")));
  const netGrid = el("div", "grid2");
  const hostF = mkField(t("tools.netHost"));
  const hostIn = el("input") as HTMLInputElement;
  hostIn.value = "www.google.com";
  hostF.append(hostIn);
  const portChk = mkField(t("tools.netPortNo"));
  const netPortIn = el("input") as HTMLInputElement;
  netPortIn.type = "number";
  netPortIn.value = "443";
  portChk.append(netPortIn);
  netGrid.append(hostF, portChk);
  netCard.append(netGrid);
  const netBtns = el("div", "actions");
  const netOut = el("pre", "lines") as HTMLElement;
  netOut.style.display = "none";
  const runNet = (kind: string) => async () => {
    const host = hostIn.value.trim();
    if (!host) return;
    netOut.style.display = "";
    if (kind === "port") {
      const r = await api.portCheck(host, Number(netPortIn.value || 443));
      netOut.textContent = r.open
        ? `open (${r.delay} ms)`
        : "closed / filtered";
      return;
    }
    netOut.textContent = "...";
    const r = await api.netTool(kind, host);
    netOut.textContent = r.detail;
  };
  for (const [kind, label] of [
    ["ping", t("tools.netPing")],
    ["trace", t("tools.netTrace")],
    ["dns", t("tools.netDns")],
    ["port", t("tools.netPort")],
  ] as [string, string][]) {
    const b = el("button", "btn", label);
    b.onclick = runNet(kind);
    netBtns.append(b);
  }
  netCard.append(netBtns, netOut);
  wrap.append(netCard);

  // ---- my IP + speed ----
  const myCard = el("div", "card");
  myCard.append(el("h2", "", t("tools.myIpTitle")));
  const myOut = el("div", "lines") as HTMLElement;
  myCard.append(myOut);
  const myBtns = el("div", "actions");
  const myBtn = el("button", "btn", t("tools.myIpRun"));
  myBtn.onclick = async () => {
    myBtn.disabled = true;
    try {
      const info = await api.myIp();
      myOut.innerHTML = "";
      for (const key of ["query", "country", "city", "isp", "as"]) {
        if (info[key]) {
          const row = el("div", "switch-row");
          row.append(el("span", "", key), el("span", "mono", String(info[key])));
          myOut.append(row);
        }
      }
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    myBtn.disabled = false;
  };
  const speedBtn = el("button", "btn", t("tools.speedRun"));
  const speedOut = el("span", "badge", "");
  speedBtn.onclick = async () => {
    speedBtn.disabled = true;
    speedOut.className = "badge";
    speedOut.textContent = "...";
    try {
      const mbps = await api.netSpeed(10_000_000);
      speedOut.className = mbps > 10 ? "badge ok" : mbps > 2 ? "badge mid" : "badge fail";
      speedOut.textContent = `${mbps.toFixed(1)} ${t("tools.speedMbps")}`;
    } catch (e: any) {
      speedOut.className = "badge fail";
      speedOut.textContent = `${e}`;
    }
    speedBtn.disabled = false;
  };
  myBtns.append(myBtn, speedBtn, speedOut);
  myCard.append(myBtns);
  wrap.append(myCard);

  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// settings view
// ---------------------------------------------------------------------------

function renderSettings() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("settings.title")));
  wrap.append(head);
  if (!settings) return;
  const s = settings;

  // search input for settings
  const searchWrapper = el("div", "field");
  const searchInput = el("input") as HTMLInputElement;
  searchInput.type = "text";
  searchInput.placeholder = t("settings.search");
  searchInput.className = "settings-search";
  searchInput.addEventListener("input", filterSettings);
  searchWrapper.append(searchInput);
  wrap.append(searchWrapper);

  function filterSettings() {
    const query = (searchInput.value || "").toLowerCase();
    const allElements = wrap.querySelectorAll(".card, .bento-box");
    allElements.forEach((el: HTMLElement) => {
      const text = (el.textContent || "").toLowerCase();
      el.style.display = text.includes(query) ? "" : "none";
    });
    // Show header always
    const headers = wrap.querySelectorAll("h1, h2");
    headers.forEach((h: HTMLElement) => {
      h.style.display = "";
    });
  }

  // Initialize: show all first, then apply filter
  filterSettings();

  // Shared expand/collapse behavior: the toggle hides the whole row (control
  // included) and its label always reflects the current state.
  const mkCollapse = (body: HTMLElement, initialCollapsed: boolean) => {
    const toggle = el("button", "btn ghost small");
    const sync = () => {
      toggle.textContent =
        body.style.display === "none" ? t("settings.expand") : t("settings.collapse");
    };
    toggle.onclick = () => {
      body.style.display = body.style.display === "none" ? "" : "none";
      sync();
    };
    if (initialCollapsed) body.style.display = "none";
    sync();
    return toggle;
  };

  const mkSwitch = (title: string, key: keyof Settings, collapsed = false) => {
    const row = el("div", "switch-row");
    const body = el("span", "switch-body");
    const titleEl = el("span", "", title);
    body.append(titleEl);
    const sw = el("button", `switch ${s[key] ? "on" : ""}`);
    sw.onclick = async () => {
      await save({ [key]: !s[key] } as any);
      (s as any)[key] = !s[key];
      sw.className = `switch ${s[key] ? "on" : ""}`;
    };
    body.append(sw);
    row.append(body);
    if (collapsed) row.append(mkCollapse(body, true));
    return row;
  };

  const mkInput = (title: string, key: keyof Settings, type = "text", collapsed = false) => {
    const f = el("div", "field");
    const body = el("div", "field-body");
    body.append(el("label", "", title));
    const i = el("input") as HTMLInputElement;
    i.type = type;
    i.value = String(s[key] ?? "");
    i.addEventListener("change", () =>
      save({ [key]: type === "number" ? Number(i.value || 0) : i.value } as any)
    );
    body.append(i);
    f.append(body);
    if (collapsed) f.append(mkCollapse(body, true));
    return f;
  };

  const mkSelect = (title: string, key: keyof Settings, options: [string, string][], collapsed = false) => {
    const f = el("div", "field");
    const body = el("div", "field-body");
    body.append(el("label", "", title));
    const sel = el("select") as HTMLSelectElement;
    for (const [k, v] of options) {
      const o = el("option", "", v) as HTMLOptionElement;
      o.value = k;
      sel.append(o);
    }
    sel.value = String(s[key] ?? "");
    sel.onchange = () => save({ [key]: sel.value } as any);
    body.append(sel);
    f.append(body);
    if (collapsed) f.append(mkCollapse(body, true));
    return f;
  };

  // connection mode card
  const modeCard = el("div", "card");
  modeCard.append(el("h2", "", t("settings.modeTitle")));
  const grid = el("div", "bento");
  const sysBox = el("div", "bento-box");
  sysBox.append(el("h3", "", t("settings.systemMode")), el("p", "muted", t("settings.systemModeDesc")));
  const tunBox = el("div", "bento-box");
  tunBox.append(el("h3", "", t("settings.tunMode")), el("p", "muted", t("settings.tunModeDesc")));
  grid.append(sysBox, tunBox);
  modeCard.append(grid);
  wrap.append(modeCard);

  const general = el("div", "card");
  general.append(el("h2", "", t("settings.general")));
  const langOptions: [string, string][] = [
    ["auto", t("settings.auto")],
    ["en", "English"],
    ["fa", "فارسی (RTL)"],
  ];
  // Language is never collapsed
  general.append(mkSelect(t("settings.language"), "language", langOptions, false));
  general.append(
    (() => {
      const f = mkSelect(t("settings.theme"), "theme", [
        ["auto", t("settings.themeAuto")],
        ["light", t("settings.themeLight")],
        ["dark", t("settings.themeDark")],
      ], true);
      // mkSelect only persists; apply the theme immediately too
      const sel = f.querySelector("select") as HTMLSelectElement;
      sel.addEventListener("change", () => applyTheme(sel.value));
      return f;
    })()
  );
  // Minimize tray and auto-connect can be collapsed as advanced options
  general.append(mkSwitch(t("settings.minimizeTray"), "minimize_to_tray", true));
  general.append(mkSwitch(t("settings.autoConnect"), "auto_connect", true));
  wrap.append(general);

  // ---------------------------------------------------------------------------
  // failover mode setting
  // ---------------------------------------------------------------------------

  const failoverCard = el("div", "card");
  failoverCard.append(el("h2", "", t("settings.failoverTitle")));
  const failoverRow = el("div", "switch-row");
  const failoverLabel = el("span", "", t("settings.failoverMode"));
  failoverRow.append(failoverLabel);
  const failoverSw = el("button", `switch ${s.failover_mode === "round-robin" || s.failover_mode === "failover" ? "on" : ""}`);
  failoverSw.onclick = async () => {
    const newMode = s.failover_mode === "off" ? "round-robin" : "off";
    await save({ failover_mode: newMode });
    (s as any).failover_mode = newMode;
    failoverSw.className = `switch ${newMode !== "off" ? "on" : ""}`;
  };
  failoverRow.append(failoverSw);
  failoverCard.append(failoverRow);
  wrap.append(failoverCard);

  const conn = el("div", "card");
  conn.append(el("h2", "", t("settings.connection")));
  const connGrid = el("div", "grid2");
  connGrid.append(
    mkInput(t("settings.socksPort"), "socks_port", "number"),
    mkInput(t("settings.httpPort"), "http_port", "number"),
    mkSelect(t("settings.loglevel"), "loglevel", [
      ["debug", "debug"],
      ["info", "info"],
      ["warning", "warning"],
      ["error", "error"],
      ["none", "none"],
    ]),
    mkInput(t("settings.remoteDns"), "remote_dns"),
    mkSelect(t("routing.domainStrategy"), "domain_strategy", [
      ["IPIfNonMatch", "IPIfNonMatch"],
      ["AsIs", "AsIs"],
      ["IPOnDemand", "IPOnDemand"],
    ])
  );
  conn.append(connGrid);
  conn.append(
    mkSwitch(t("settings.sniffing"), "sniffing"),
    mkSwitch(t("settings.allowInsecure"), "allow_insecure"),
    mkSwitch(t("settings.mux"), "mux"),
    mkSwitch(t("settings.ipv6"), "ipv6"),
    mkSwitch(t("settings.proxySharing"), "proxy_sharing")
  );
  wrap.append(conn);

  const frag = el("div", "card");
  frag.append(el("h2", "", t("settings.fragment")));
  frag.append(mkSwitch(t("settings.fragOn"), "fragment_enabled"));
  const fragGrid = el("div", "grid3");
  fragGrid.append(
    mkInput(t("settings.pack"), "fragment_packets"),
    mkInput(t("settings.length"), "fragment_length"),
    mkInput(t("settings.interval"), "fragment_interval")
  );
  frag.append(fragGrid);

  const cdnGrid = el("div", "grid2");
  cdnGrid.append(
    mkSelect(t("settings.cdn"), "cdn_mode", [
      ["off", t("settings.cdnOff")],
      ["manual", t("settings.cdnManual")],
    ]),
    mkInput(t("settings.cdnIp"), "cdn_ip")
  );
  frag.append(cdnGrid);
  wrap.append(frag);

  // ECH + domain fronting (stealth card)
  const stealth = el("div", "card");
  stealth.append(el("h2", "", t("settings.echTitle")));
  stealth.append(mkSwitch(t("settings.echOn"), "ech_enabled"));
  stealth.append(mkInput(t("settings.echConfig"), "ech_config"));

  const frontHead = el("h2", "", t("settings.frontTitle"));
  stealth.append(frontHead);
  stealth.append(mkSwitch(t("settings.frontOn"), "front_enabled"));
  stealth.append(mkInput(t("settings.frontHost"), "front_host"));
  wrap.append(stealth);

  // psiphon ports card (full controls live on the Psiphon page)
  const pCard = el("div", "card");
  pCard.append(el("h2", "", t("settings.psiphonCard")));
  const pGrid = el("div", "grid2");
  pGrid.append(
    mkInput(t("psiphon.socksPort"), "psiphon_socks_port", "number"),
    mkInput(t("psiphon.httpPort"), "psiphon_http_port", "number")
  );
  pCard.append(pGrid);
  wrap.append(pCard);

  renderRequirementsCard(wrap);

  wrap.append(el("p", "muted", t("settings.danger")));
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// logs view
// ---------------------------------------------------------------------------

let trafficUp = 0;
let trafficDown = 0;

function renderLogs() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("logs.title")));
  const actions = el("div", "actions");
  const btnClear = el("button", "btn", t("logs.clear"));
  btnClear.onclick = async () => {
    await api.clearLogs();
    logBox!.textContent = "";
  };
  actions.append(btnClear);
  head.append(actions);
  wrap.append(head);

  const trafficCard = el("div", "card");
  trafficCard.append(el("h2", "", t("logs.session")));
  const trow = el("div", "traffic-detail");
  const upEl = el("div", "traffic-item");
  upEl.append(el("span", "muted", t("logs.up")), el("span", "traffic-num", fmtBytes(trafficUp)));
  const downEl = el("div", "traffic-item");
  downEl.append(el("span", "muted", t("logs.down")), el("span", "traffic-num", fmtBytes(trafficDown)));
  trow.append(downEl, upEl);
  trafficCard.append(trow);
  wrap.append(trafficCard);

  const logBox = el("pre", "logbox", "");
  (logBox as any).id = "logbox";
  api.getLogs().then((lines) => {
    logBox.textContent = lines.join("\n") || t("logs.empty");
    logBox.scrollTop = logBox.scrollHeight;
  });
  wrap.append(logBox);
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// about view
// ---------------------------------------------------------------------------

function renderAbout() {
  const wrap = el("div", "view");
  const card = el("div", "card about");
  card.append(el("h1", "", t("about.title")));
  card.append(el("p", "", t("about.body")));
  const links = el("div", "links");
  const mk = (label: string, url: string) => {
    const a = el("a", "link");
    a.href = "#";
    a.textContent = label;
    a.onclick = (e: MouseEvent) => {
      e.preventDefault();
      // @ts-ignore injected by withGlobalTauri
      window.__TAURI__?.opener?.openUrl?.(url);
      if (!(window as any).__TAURI__) window.open(url, "_blank");
    };
    links.append(a);
  };
  mk(t("about.telegram"), "https://t.me/mahsa_net");
  mk(t("about.repo"), "https://github.com/pourjahani/MahsaDesk");
  mk(t("about.android"), "https://github.com/GFW-knocker/MahsaNG");
  card.append(el("h2", "", t("about.links")), links);
  card.append(el("p", "muted", t("about.donation")));
  api.version().then((v) => {
    const ver = el("div", "ver");
    ver.append(el("span", "muted", `${t("about.version")}: `), el("span", "badge", "v" + v));
    card.append(ver);
  });
  wrap.append(card);
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// connect / disconnect
// ---------------------------------------------------------------------------

async function doConnect(mode: string) {
  statusText.textContent = t("status.connecting");
  try {
    await api.connect(mode);
  } catch (e: any) {
    alert(`${t("connectErr")}: ${e}`);
  }
  await refreshData();
  updateStatus();
}

async function doDisconnect() {
  try {
    await api.disconnect();
  } catch { /* ok */ }
  await refreshData();
  updateStatus();
}

// ---------------------------------------------------------------------------
// traffic + log listeners
// ---------------------------------------------------------------------------

function listenTraffic() {
  try {
    // @ts-ignore
    const { listen } = (window as any).__TAURI__.event;
    listen("traffic", (e: any) => {
      const { up, down } = e.payload;
      trafficUp = up;
      trafficDown = down;
      if (trafficEl) {
        trafficEl.textContent = `- ${fmtBytes(down)} / ${fmtBytes(up)}`;
      }
      const upNum = document.querySelector(".traffic-num") as HTMLElement | null;
      if (upNum && currentView === "logs") {
        const els = document.querySelectorAll(".traffic-num");
        if (els[0]) els[0].textContent = fmtBytes(down);
        if (els[1]) els[1].textContent = fmtBytes(up);
      }
    });
    listen("core-log", (e: any) => {
      const box = document.getElementById("logbox");
      if (box) {
        box.textContent += e.payload.line + "\n";
        box.scrollTop = box.scrollHeight;
      }
    });
  } catch { /* not on tauri bridge during dev */ }
}

// ---------------------------------------------------------------------------
// boot
// ---------------------------------------------------------------------------

function detectLang(): Lang {
  const stored = localStorage.getItem("mahsa-lang");
  if (stored === "fa" || stored === "en") return stored;
  const sys = navigator.language?.toLowerCase() || "en";
  if (sys.startsWith("fa")) return "fa";
  return "en";
}

async function init() {
  const sysLang = detectLang();
  setLang(sysLang);
  await refreshData();
  if (settings && ["en", "fa"].includes(settings.language)) {
    setLang(settings.language as Lang);
    localStorage.setItem("mahsa-lang", settings.language);
  }
  // apply the persisted theme before first paint of the layout
  applyTheme(settings?.theme);
  buildLayout();
  renderView();
  updateStatus();
  listenTraffic();
  // release check after first paint so it never delays startup
  setTimeout(checkAppUpdateOnBoot, 1500);
  setInterval(updateStatus, 4000);
  setInterval(async () => {
    await refreshData();
    if (currentView === "servers") {
      // lightweight refresh of delay badges only
      const list = document.querySelector(".server-list");
      if (list && !document.querySelector(".modal-back")) renderView();
    }
  }, 8000);
}

init();

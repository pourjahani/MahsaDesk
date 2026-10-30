import "./style.css";
import { api, readQrFromClipboard, type ServerProfile, type Subscription, type Settings, type DnsResult, type IpResult, type ReqInfo, type AppUpdateInfo, type DoctorCheck, type ShareInfo } from "./api";
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
    share: '<path d="M10 14a4 4 0 0 1 0-5.7l2-2a4 4 0 0 1 5.7 5.7l-1 1M14 10a4 4 0 0 1 0 5.7l-2 2a4 4 0 0 1-5.7-5.7l1-1" fill="none" stroke-width="2.2" stroke-linecap="round"/>',
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
  const btnSystem = el("button", "btn primary", t("status.connSystem"));
  btnSystem.title = t("settings.systemMode");
  btnSystem.onclick = () => doConnect("system");
  const btnTun = el("button", "btn", t("status.connTun"));
  btnTun.title = t("status.tunMode");
  btnTun.onclick = () => doConnect("tun");
  const btnPsiphon = el("button", "btn", t("status.connPsiphon"));
  btnPsiphon.title = t("status.psiphonOnly");
  btnPsiphon.onclick = () => doConnect("psiphon-only");
  btnRow.append(btnSystem, btnTun, btnPsiphon);
  const btnStop = el("button", "btn ghost", "Stop");
  btnStop.id = "btn-stop";
  btnStop.style.display = "none";
  btnStop.onclick = () => doDisconnect();
  trafficEl = el("div", "traffic", "");
  connectCard.append(row, trafficEl, btnRow, btnStop);
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

/** Apply a language immediately: persist, repaint layout + current view. */
function applyLang(l: Lang) {
  setLang(l);
  localStorage.setItem("mahsa-lang", l);
  buildLayout();
  renderView();
  updateStatus();
}

// ---------------------------------------------------------------------------
// new-release banner (startup check, requirements-card pattern)
// ---------------------------------------------------------------------------

function openExternal(url: string) {
  // @ts-ignore injected by withGlobalTauri
  window.__TAURI__?.opener?.openUrl?.(url);
  if (!(window as any).__TAURI__) window.open(url, "_blank");
}

/** Download the asset that matches this build, then install and restart. */
async function runAppUpdate(u: AppUpdateInfo, btn: HTMLButtonElement) {
  const url = u.portable ? u.portable_url || u.download_url : u.installer_url || u.download_url;
  if (!url) {
    openExternal(u.release_url);
    return;
  }
  const sha = (u.portable ? u.portable_sha256 : u.installer_sha256) || null;
  btn.disabled = true;
  btn.textContent = t("update.downloading");
  const unlisten = await api.onAppUpdateProgress((downloaded, total) => {
    const pct = total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null;
    btn.textContent = pct == null ? t("update.downloading") : `${t("update.downloading")} ${pct}%`;
  });
  try {
    const path = await api.appUpdateDownload(url, u.latest_version, sha);
    btn.textContent = t("update.preparing");
    // the helper closes this app, installs and relaunches it
    await api.appUpdateInstall(path, !!u.portable);
  } catch (e: any) {
    btn.disabled = false;
    btn.textContent = t("update.install");
    alert(`${t("update.installFailed")}: ${e}`);
  } finally {
    unlisten();
  }
}

function buildUpdateBanner(): HTMLElement {
  const info = updateInfo!;
  const card = el("div", "card update-banner");
  card.append(
    el("h2", "", t("update.title")),
    el("p", "", `${t("update.current")}: ${info.current_version}  ->  ${t("update.latest")}: ${info.latest_version}`)
  );
  const actions = el("div", "actions");
  if (info.installer_url || info.portable_url || info.download_url) {
    const dl = el("button", "btn primary", t("update.install"));
    dl.onclick = () => runAppUpdate(info, dl);
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

// ---------------------------------------------------------------------------
// servers view state (#12 sort/filter/search, #14 bulk selection, #16 share)
// ---------------------------------------------------------------------------

let srvQuery = "";
let srvSort = "default";
let srvProto = "";
let srvSub = "";
let srvSelectMode = false;
let srvSelected = new Set<string>();

function visibleServers(): ServerProfile[] {
  let list = servers.slice();
  if (srvQuery.trim()) {
    const q = srvQuery.trim().toLowerCase();
    list = list.filter(
      (p) =>
        (p.remark || "").toLowerCase().includes(q) ||
        p.address.toLowerCase().includes(q) ||
        p.protocol.toLowerCase().includes(q)
    );
  }
  if (srvProto) list = list.filter((p) => p.protocol === srvProto);
  if (srvSub) {
    list =
      srvSub === "-"
        ? list.filter((p) => !p.sub_id)
        : list.filter((p) => p.sub_id === srvSub);
  }
  const far = 1_000_000_000;
  switch (srvSort) {
    case "delay":
      list.sort((a, b) => (a.delay ?? far) - (b.delay ?? far));
      break;
    case "name":
      list.sort((a, b) =>
        (a.remark || a.address).localeCompare(b.remark || b.address)
      );
      break;
    case "protocol":
      list.sort(
        (a, b) =>
          a.protocol.localeCompare(b.protocol) ||
          (a.remark || "").localeCompare(b.remark || "")
      );
      break;
    case "sub":
      list.sort((a, b) => a.sub_id.localeCompare(b.sub_id));
      break;
    case "newest":
      list.sort((a, b) => b.id.localeCompare(a.id));
      break;
  }
  return list;
}

function serverToolbar(): HTMLElement {
  const bar = el("div", "srv-toolbar");
  const search = el("input", "srv-search") as HTMLInputElement;
  search.placeholder = t("servers.search");
  search.value = srvQuery;
  search.addEventListener("input", () => {
    srvQuery = search.value;
    renderView();
    // the view is rebuilt on every keystroke: restore focus at the end
    const again = document.querySelector(".srv-search") as HTMLInputElement | null;
    if (again) {
      again.focus();
      again.setSelectionRange(again.value.length, again.value.length);
    }
  });
  bar.append(search);

  const sort = el("select", "srv-sort") as HTMLSelectElement;
  for (const [k, label] of [
    ["default", t("servers.sort.default")],
    ["delay", t("servers.sort.delay")],
    ["name", t("servers.sort.name")],
    ["protocol", t("servers.sort.protocol")],
    ["sub", t("servers.sort.sub")],
    ["newest", t("servers.sort.newest")],
  ] as [string, string][]) {
    const o = el("option", "", label) as HTMLOptionElement;
    o.value = k;
    sort.append(o);
  }
  sort.value = srvSort;
  sort.onchange = () => {
    srvSort = sort.value;
    renderView();
  };
  bar.append(sort);

  // protocol filter chips (#12)
  const protos = [...new Set(servers.map((p) => p.protocol))].sort();
  for (const proto of protos) {
    const chip = el(
      "button",
      `chip filter-chip ${srvProto === proto ? "on" : ""}`,
      proto
    );
    chip.onclick = () => {
      srvProto = srvProto === proto ? "" : proto;
      renderView();
    };
    bar.append(chip);
  }

  // subscription filter
  const subSel = el("select", "srv-sort") as HTMLSelectElement;
  const allOpt = el("option", "", t("servers.filter.allSubs")) as HTMLOptionElement;
  allOpt.value = "";
  subSel.append(allOpt);
  const manualOpt = el("option", "", t("servers.filter.manual")) as HTMLOptionElement;
  manualOpt.value = "-";
  subSel.append(manualOpt);
  for (const s of subs) {
    const o = el("option", "", s.name) as HTMLOptionElement;
    o.value = s.id;
    subSel.append(o);
  }
  subSel.value = srvSub;
  subSel.onchange = () => {
    srvSub = subSel.value;
    renderView();
  };
  bar.append(subSel);

  return bar;
}

function bulkBar(): HTMLElement {
  const bar = el("div", "bulk-bar");
  const ids = [...srvSelected];
  const shown = visibleServers();
  bar.append(
    el(
      "span",
      "bulk-count",
      t("servers.selectedCount").replace("{n}", String(ids.length))
    )
  );

  const btnAll = el("button", "btn", t("servers.selectAll"));
  btnAll.onclick = () => {
    for (const p of shown) srvSelected.add(p.id);
    renderView();
  };
  const btnNone = el("button", "btn ghost", t("servers.clearSelection"));
  btnNone.onclick = () => {
    srvSelected.clear();
    renderView();
  };
  const btnTest = el("button", "btn", t("servers.testSelected"));
  btnTest.disabled = ids.length === 0;
  btnTest.onclick = async () => {
    btnTest.disabled = true;
    // one batched backend call: parallel pings instead of a round-trip each
    await api.testAllDelays(ids);
    servers = await api.listServers();
    renderView();
  };
  const btnReal = el("button", "btn", t("servers.realSelected"));
  btnReal.disabled = ids.length === 0;
  btnReal.onclick = async () => {
    btnReal.disabled = true;
    try {
      await api.testAllReal(ids);
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    servers = await api.listServers();
    renderView();
  };
  const btnShare = el("button", "btn", t("servers.shareSelected"));
  btnShare.disabled = ids.length === 0;
  btnShare.onclick = () => shareDialog(ids);
  const btnExport = el("button", "btn", t("servers.exportSelected"));
  btnExport.disabled = ids.length === 0;
  btnExport.onclick = async () => {
    try {
      const links = await api.exportLinks(ids);
      await navigator.clipboard.writeText(links.join("\n"));
      btnExport.textContent = t("servers.copied");
      setTimeout(() => {
        btnExport.textContent = t("servers.exportSelected");
      }, 1500);
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  const btnDel = el("button", "btn danger", t("servers.deleteSelected"));
  btnDel.disabled = ids.length === 0;
  btnDel.onclick = async () => {
    if (!confirm(t("servers.deleteConfirm").replace("{n}", String(ids.length)))) {
      return;
    }
    await api.deleteServers(ids);
    srvSelected.clear();
    servers = await api.listServers();
    renderView();
  };
  bar.append(btnAll, btnNone, btnTest, btnReal, btnExport, btnShare, btnDel);
  return bar;
}

/** Share the pool as a subscription URL (#16): start form or live endpoint. */
async function shareDialog(ids?: string[]) {
  const info = await api.shareStatus().catch(() => null);
  const dlg = el("div", "modal-back");
  const box = el("div", "modal");
  box.append(el("h2", "", ids ? t("servers.shareSelected") : t("servers.share")));

  if (info) {
    box.append(el("p", "mono share-url", info.url));
    const img = el("img", "qr-img") as HTMLImageElement;
    img.style.display = "none";
    box.append(img);
    try {
      const QR = (await import("qrcode")).default;
      img.src = await QR.toDataURL(info.url, { width: 260, margin: 1 });
      img.style.display = "";
    } catch {
      // QR rendering is optional; the URL below is always copyable
    }
    box.append(el("p", "muted", t("share.note")));
    if (info.scope) {
      box.append(
        el("p", "muted", t("share.scopeNote").replace("{n}", String(info.scope.length)))
      );
    }
    const row = el("div", "modal-actions");
    const copy = el("button", "btn primary", t("servers.copy"));
    copy.onclick = async () => {
      await navigator.clipboard.writeText(info.url);
      copy.textContent = t("servers.copied");
    };
    const stop = el("button", "btn", t("share.stop"));
    stop.onclick = async () => {
      await api.shareStop();
      dlg.remove();
    };
    const close = el("button", "btn ghost", t("servers.cancel"));
    close.onclick = () => dlg.remove();
    row.append(copy, stop, close);
    box.append(row);
  } else {
    const form = el("div", "form");
    const pf = el("div", "field");
    pf.append(el("label", "", t("share.port")));
    const port = el("input") as HTMLInputElement;
    port.type = "number";
    port.value = String(settings?.share_port ?? 18081);
    pf.append(port);
    const lanRow = el("div", "switch-row");
    lanRow.append(el("span", "", t("share.lan")));
    const sw = el("button", `switch ${settings?.share_lan ? "on" : ""}`);
    let lan = !!settings?.share_lan;
    sw.onclick = () => {
      lan = !lan;
      sw.className = `switch ${lan ? "on" : ""}`;
    };
    lanRow.append(sw);
    form.append(pf, lanRow);
    box.append(form, el("p", "muted", t("share.startNote")));
    const row = el("div", "modal-actions");
    const cancel = el("button", "btn ghost", t("servers.cancel"));
    cancel.onclick = () => dlg.remove();
    const start = el("button", "btn primary", t("share.start"));
    start.onclick = async () => {
      try {
        const p = Math.max(1, Math.min(65535, Number(port.value || 18081)));
        await save({ share_port: p, share_lan: lan });
        await api.shareStart(p, lan, ids);
        dlg.remove();
        shareDialog(ids);
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
      }
    };
    row.append(cancel, start);
    box.append(row);
  }

  dlg.append(box);
  document.body.append(dlg);
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
  // #16 share + #14 selection toggle
  const btnShare = el("button", "btn", t("servers.share"));
  btnShare.prepend(icon("share"));
  btnShare.onclick = () => shareDialog();
  const btnSelect = el(
    "button",
    `btn ${srvSelectMode ? "primary" : ""}`,
    srvSelectMode ? t("servers.cancelSelect") : t("servers.select")
  );
  btnSelect.prepend(icon("check"));
  btnSelect.onclick = () => {
    srvSelectMode = !srvSelectMode;
    srvSelected.clear();
    renderView();
  };
  actions.append(btnImport, btnPing, btnReal, btnValidate, btnShare, btnSelect);
  head.append(actions);
  wrap.append(head);
  wrap.append(serverToolbar());
  if (srvSelectMode) wrap.append(bulkBar());

  const activeId = settings?.active_server || "";
  if (servers.length === 0) {
    wrap.append(el("div", "empty", t("servers.noServers")));
    mainEl.append(wrap);
    return;
  }

  const shown = visibleServers();
  if (shown.length === 0) {
    wrap.append(el("div", "empty", t("servers.noMatches")));
    mainEl.append(wrap);
    return;
  }

  const list = el("div", "server-list");
  for (const p of shown) {
    const item = el("div", "server-row");
    if (p.id === activeId) item.classList.add("active");

    if (srvSelectMode) {
      const cb = el("input", "srv-check") as HTMLInputElement;
      cb.type = "checkbox";
      cb.checked = srvSelected.has(p.id);
      cb.onclick = (ev: Event) => {
        ev.stopPropagation();
        if (cb.checked) srvSelected.add(p.id);
        else srvSelected.delete(p.id);
        renderView();
      };
      item.append(cb);
    }

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
      if (srvSelectMode) return;
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

/**
 * #10: curated community sources. Every URL below was fetched and confirmed
 * to return real share-link payloads, with commits within the last months
 * (checked 2026-10-22). MahsaNet's own per-ISP feeds come first.
 */
const COMMUNITY_SOURCES: { name: string; url: string }[] = [
  {
    name: "MahsaNet MCI",
    url: "https://raw.githubusercontent.com/mahsanet/MahsaFreeConfig/main/mci/sub_1.txt",
  },
  {
    name: "MahsaNet Irancell (MTN)",
    url: "https://raw.githubusercontent.com/mahsanet/MahsaFreeConfig/main/mtn/sub_1.txt",
  },
  {
    name: "0xRadikal top 100",
    url: "https://raw.githubusercontent.com/0xRadikal/Free-v2ray-Configs/main/top100.txt",
  },
  {
    name: "0xRadikal verified",
    url: "https://raw.githubusercontent.com/0xRadikal/Free-v2ray-Configs/main/verified/configs_base64.txt",
  },
  {
    name: "MatinGhanbari all protocols",
    url: "https://raw.githubusercontent.com/MatinGhanbari/v2ray-configs/main/subscriptions/v2ray/all_sub.txt",
  },
  {
    name: "MatinGhanbari super",
    url: "https://raw.githubusercontent.com/MatinGhanbari/v2ray-configs/main/subscriptions/v2ray/super-sub.txt",
  },
  {
    name: "MatinGhanbari VLESS",
    url: "https://raw.githubusercontent.com/MatinGhanbari/v2ray-configs/main/subscriptions/filtered/subs/vless.txt",
  },
  {
    name: "MatinGhanbari Hysteria2",
    url: "https://raw.githubusercontent.com/MatinGhanbari/v2ray-configs/main/subscriptions/filtered/subs/hysteria2.txt",
  },
  {
    name: "barry-far Sub1",
    url: "https://raw.githubusercontent.com/barry-far/V2ray-config/main/Sub1.txt",
  },
  {
    name: "MahanKenway mix",
    url: "https://raw.githubusercontent.com/MahanKenway/Freedom-V2Ray/main/configs/mix_sub.txt",
  },
];

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

  // #10: community sources card - works even on a fresh install (shown above
  // the empty state so new users get servers in one tap)
  const comm = el("div", "card");
  comm.append(el("h2", "", t("subs.communityTitle")));
  comm.append(el("p", "muted", t("subs.communityNote")));
  const commList = el("div", "lines");
  for (const src of COMMUNITY_SOURCES) {
    const row = el("div", "switch-row");
    const info = el("div", "srv-info");
    info.append(
      el("span", "srv-name", src.name),
      el("div", "srv-meta", src.url)
    );
    row.append(info);
    const already = subs.some((s) => s.url === src.url);
    const btn = el(
      "button",
      "btn",
      already ? t("subs.added") : t("subs.addOne")
    );
    btn.disabled = already;
    btn.onclick = async () => {
      btn.disabled = true;
      btn.textContent = t("requirements.updating");
      try {
        await api.addSubscription(src.name, src.url, 12, true);
        await refreshData();
        renderView();
      } catch (e: any) {
        const saved = (await api.listSubscriptions().catch(() => [])).some(
          (sub) => sub.url === src.url
        );
        if (saved) {
          await refreshData();
          renderView();
        }
        alert(`${t("error")}: ${e}`);
        btn.disabled = false;
        btn.textContent = t("subs.addOne");
      }
    };
    row.append(btn);
    commList.append(row);
  }
  comm.append(commList);
  const missingSources = COMMUNITY_SOURCES.filter(
    (c) => !subs.some((s) => s.url === c.url)
  );
  if (missingSources.length > 0) {
    const addAll = el("button", "btn primary", t("subs.addAll"));
    addAll.onclick = async () => {
      addAll.disabled = true;
      addAll.textContent = t("requirements.updating");
      try {
        for (const src of missingSources) {
          await api.addSubscription(src.name, src.url, 12, false);
        }
        const fresh = await api.listSubscriptions();
        for (const s of fresh.filter((x) =>
          missingSources.some((m) => m.url === x.url)
        )) {
          await api.updateSubscription(s.id).catch(() => 0);
        }
        await refreshData();
        renderView();
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
        addAll.disabled = false;
        addAll.textContent = t("subs.addAll");
      }
    };
    comm.append(addAll);
  }
  wrap.append(comm);

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
      const saved = !s && (await api.listSubscriptions().catch(() => [])).some(
        (sub) => sub.url === inUrl.value.trim()
      );
      if (saved) {
        dlg.remove();
        await refreshData();
        renderView();
      }
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

  // #22: per-ISP anti-filtering presets (honest about what they change)
  const ispCard = el("div", "card");
  ispCard.append(el("h2", "", t("isp.title")));
  ispCard.append(el("p", "muted", t("isp.note")));
  const ispRow = el("div", "actions");
  api
    .myIp()
    .then(async (info: any) => {
      const id = await api.detectIsp(String(info?.isp ?? ""), String(info?.org ?? ""));
      if (id && currentView === "routing") {
        const chip = el("span", "badge mid", `${t("isp.detected")}: ${t("isp." + id)}`);
        ispRow.prepend(chip);
      }
    })
    .catch(() => { /* offline: no preselect */ });
  api.ispPresets().then((presets) => {
    for (const p of presets) {
      const b = el("button", "btn", t("isp." + p.id));
      b.title = t("isp.tip" + p.id.charAt(0).toUpperCase() + p.id.slice(1));
      const active =
        !!settings &&
        settings.fragment_enabled === p.fragment_enabled &&
        settings.fragment_packets === p.packets &&
        settings.fragment_length === p.length &&
        settings.fragment_interval === p.interval;
      if (active) b.classList.add("primary");
      b.onclick = async () => {
        await save({
          fragment_enabled: p.fragment_enabled,
          fragment_packets: p.packets,
          fragment_length: p.length,
          fragment_interval: p.interval,
        });
        toast(t("isp.applied").replace("{n}", t("isp." + p.id)));
        renderView();
      };
      ispRow.append(b);
    }
  });
  ispCard.append(ispRow);
  const fragNow = el("p", "muted mono");
  fragNow.textContent = `fragment: ${
    settings?.fragment_enabled
      ? `${settings.fragment_packets} / ${settings.fragment_length} / ${settings.fragment_interval}`
      : "off"
  }`;
  ispCard.append(fragNow);
  wrap.append(ispCard);

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

/** Engine files are locked while cores run: ask, disconnect, then upgrade. */
async function confirmEngineUpdate(): Promise<boolean> {
  try {
    const st = await api.status();
    if (st.running) {
      if (!confirm(t("requirements.disconnectPrompt"))) return false;
      await api.disconnect();
      updateStatus();
    }
  } catch {
    // status unavailable: let the backend enforce its own guard
  }
  return true;
}

async function renderRequirementsCard(wrap: HTMLElement) {
  const card = el("div", "card");
  card.append(el("h2", "", t("requirements.title")));
  card.append(el("p", "muted", t("requirements.note")));
  const checkRow = el("div", "actions");
  const checkBtn = el("button", "btn primary", t("requirements.check"));
  const allBtn = el("button", "btn", t("requirements.updateAll"));
  checkRow.append(checkBtn, allBtn);
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
      // #25: green badge when the on-disk hash matches the pinned marker
      if (it.exists) {
        const v = el("span", `badge ${it.verified ? "ok" : ""}`);
        v.textContent = it.verified ? `✓ ${t("requirements.verified")}` : t("requirements.notVerified");
        v.title = it.verified ? t("requirements.verifiedTitle") : t("requirements.notVerifiedTitle");
        right.append(v);
      }
      if (it.exists && it.installed_version) {
        right.append(el("span", "badge", `${t("requirements.version")} ${it.installed_version.slice(0, 40)}`));
      }
      if (it.available_version && it.update_available) {
        right.append(el("span", "badge mid", t("requirements.updateFound")));
      }
      if (it.exists) {
        right.append(el("span", "badge", fmtBytes(it.size)));
      }
      const btn = el("button", "btn", t("requirements.upgrade"));
      btn.onclick = async () => {
        if (!(await confirmEngineUpdate())) return;
        btn.disabled = true;
        btn.textContent = t("requirements.updating");
        try {
          await api.requirementsUpgrade(it.key);
          paint(await api.requirementsStatus(false));
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
    allBtn.disabled = false;
    allBtn.textContent = t("requirements.updateAll");
  };

  // remote=false: local presence + pinned hashes only (no GitHub rate limit);
  // the Check button performs the upstream comparison.
  const runCheck = async (remote: boolean) => {
    checkBtn.disabled = true;
    checkBtn.textContent = t("requirements.checking");
    try {
      paint(await api.requirementsStatus(remote));
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
      checkBtn.disabled = false;
      checkBtn.textContent = t("requirements.check");
    }
  };
  checkBtn.onclick = () => runCheck(true);

  allBtn.onclick = async () => {
    allBtn.disabled = true;
    allBtn.textContent = t("requirements.checking");
    try {
      const items = await api.requirementsStatus(true);
      const todo = items.filter((i) => i.update_available || !i.exists);
      if (todo.length === 0) {
        alert(t("requirements.updated"));
      } else if (await confirmEngineUpdate()) {
        allBtn.textContent = t("requirements.updating");
        // one failing engine must not abort the rest of the batch
        const failed: string[] = [];
        for (const it of todo) {
          try {
            await api.requirementsUpgrade(it.key);
          } catch (e: any) {
            failed.push(`${it.key}: ${e}`);
          }
        }
        if (failed.length > 0) {
          alert(`${t("error")}:\n${failed.join("\n")}`);
        }
      }
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
    runCheck(false);
  };

  runCheck(false);
}

/** App self-update card on the Engines view. */
async function renderAppUpdateCard(wrap: HTMLElement) {
  const card = el("div", "card");
  card.append(el("h2", "", t("update.appTitle")));
  const info = el("p", "muted", "...");
  card.append(info);
  const actions = el("div", "actions");
  card.append(actions);
  wrap.append(card);
  try {
    const u = updateInfo ?? (await api.appUpdateCheck());
    updateInfo = u;
    if (u.update_available) {
      info.textContent = `${t("update.current")}: ${u.current_version}  ->  ${t("update.latest")}: ${u.latest_version}`;
      if (u.installer_url || u.portable_url || u.download_url) {
        const dl = el("button", "btn primary", t("update.install"));
        dl.onclick = () => runAppUpdate(u, dl);
        actions.append(dl);
      }
    } else {
      info.textContent = `${t("update.current")}: ${u.current_version} - ${t("update.upToDate")}`;
    }
    const page = el("button", "btn", t("update.release"));
    page.onclick = () => openExternal(u.release_url);
    actions.append(page);
    const check = el("button", "btn", t("update.check"));
    check.onclick = async () => {
      check.disabled = true;
      info.textContent = t("update.checking");
      try {
        updateInfo = await api.appUpdateCheck();
        updateDismissed = false;
        renderView();
      } catch {
        info.textContent = t("update.checkFailed");
        check.disabled = false;
      }
    };
    actions.append(check);
  } catch {
    info.textContent = t("update.checkFailed");
  }
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

  // engines live here now: requirements card (#25 badges) + app self-update
  renderRequirementsCard(wrap);
  renderAppUpdateCard(wrap);

  wrap.append(el("p", "muted", t("assets.note")));

  const card = el("div", "card");
  card.append(el("h2", "", t("routing.geoTitle")));
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

// ---------------------------------------------------------------------------
// connection doctor (#21)
// ---------------------------------------------------------------------------

let doctorResults: DoctorCheck[] | null = null;
let doctorBusy = false;

async function runDoctor(rerender: boolean) {
  doctorBusy = true;
  doctorResults = null;
  if (rerender && currentView === "connection") renderView();
  try {
    doctorResults = await api.connectionDoctor();
  } catch (e: any) {
    doctorResults = [{ id: "engine", ok: false, detail: String(e) }];
  } finally {
    doctorBusy = false;
    if (rerender && currentView === "connection") renderView();
  }
}

function doctorCard(): HTMLElement {
  const card = el("div", "card");
  card.append(el("h2", "", t("doctor.title")));
  const row = el("div", "actions");
  const btn = el("button", "btn primary", doctorBusy ? t("doctor.running") : t("doctor.run"));
  btn.disabled = doctorBusy;
  btn.onclick = () => runDoctor(true);
  row.append(btn);
  card.append(row);

  const list = el("div", "lines");
  if (doctorBusy) {
    list.append(el("p", "muted", t("doctor.running")));
  } else if (!doctorResults) {
    list.append(el("p", "muted", t("doctor.hint")));
  } else {
    for (const c of doctorResults) {
      const r = el("div", "switch-row");
      r.append(
        el("span", "", t(`doctor.check.${c.id}`)),
        el("span", "mono doctor-detail", c.detail),
        el("span", `badge ${c.ok ? "ok" : "fail"}`, c.ok ? "✓" : "✕")
      );
      list.append(r);
    }
  }
  card.append(list);
  return card;
}

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

  // #21: connection doctor checklist
  wrap.append(doctorCard());
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
    ["akamai", "Akamai"],
    ["fastly", "Fastly"],
    ["bunny", "Bunny"],
    ["warp", "Cloudflare WARP"],
    ["masque", "Cloudflare MASQUE"],
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
  const udpNote = el("p", "muted", t("tools.ipUdpNote"));
  udpNote.style.display = "none";
  presetSel.onchange = () => {
    customTa.style.display = presetSel.value === "custom" ? "" : "none";
    const udp = presetSel.value === "warp" || presetSel.value === "masque";
    udpNote.style.display = udp ? "" : "none";
  };
  ipCard.append(customTa, udpNote);
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
  // Language is never collapsed; apply the change immediately too
  general.append(
    (() => {
      const f = mkSelect(t("settings.language"), "language", langOptions, false);
      const sel = f.querySelector("select") as HTMLSelectElement;
      sel.addEventListener("change", () => {
        const sys = (navigator.language || "en").toLowerCase();
        const v: Lang =
          sel.value === "fa" ? "fa" : sel.value === "en" ? "en" : sys.startsWith("fa") ? "fa" : "en";
        if (sel.value === "auto") localStorage.removeItem("mahsa-lang");
        applyLang(v);
      });
      return f;
    })()
  );
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
    mkInput(t("settings.downloadProxy"), "download_proxy"),
    mkSelect(t("routing.domainStrategy"), "domain_strategy", [
      ["IPIfNonMatch", "IPIfNonMatch"],
      ["AsIs", "AsIs"],
      ["IPOnDemand", "IPOnDemand"],
    ])
  );
  conn.append(connGrid);
  conn.append(el("p", "muted", t("settings.downloadProxyHint")));
  conn.append(
    mkSwitch(t("settings.sniffing"), "sniffing"),
    mkSwitch(t("settings.allowInsecure"), "allow_insecure"),
    mkSwitch(t("settings.mux"), "mux"),
    mkSwitch(t("settings.ipv6"), "ipv6"),
    mkSwitch(t("settings.proxySharing"), "proxy_sharing"),
    el("p", "muted", t("settings.proxySharingWarn"))
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

  wrap.append(el("p", "muted", t("settings.danger")));
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// logs view
// ---------------------------------------------------------------------------

let trafficUp = 0;
let trafficDown = 0;

// ---------------------------------------------------------------------------
// logs view state (#23: level filter, search, copy/export, crash history)
// ---------------------------------------------------------------------------

let logLevel = ""; // "" | error | warning | info
let logQuery = "";

function logLevelOf(line: string): string {
  const l = line.toLowerCase();
  if (
    /(error|failed|fatal|cannot |refused|panic|exited|denied|timed out|timeout)/.test(l)
  ) {
    return "error";
  }
  if (l.includes("warn")) return "warning";
  return "info";
}

/** Small transient message pinned to the bottom of the window. */
function toast(msg: string) {
  const node = el("div", "toast", msg);
  document.body.append(node);
  setTimeout(() => node.remove(), 4500);
}

function renderLogs() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("logs.title")));
  const actions = el("div", "actions");

  const btnCopy = el("button", "btn", t("logs.copyAll"));
  btnCopy.onclick = async () => {
    try {
      const lines = await api.getLogs();
      await navigator.clipboard.writeText(lines.join("\n"));
      btnCopy.textContent = t("servers.copied");
      setTimeout(() => (btnCopy.textContent = t("logs.copyAll")), 1500);
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  const btnExport = el("button", "btn", t("logs.export"));
  btnExport.onclick = async () => {
    try {
      const lines = await api.getLogs();
      const path = await api.exportLogs(lines.join("\n"));
      alert(`${t("logs.exported")}: ${path}`);
    } catch (e: any) {
      alert(`${t("error")}: ${e}`);
    }
  };
  const btnClear = el("button", "btn ghost", t("logs.clear"));
  btnClear.onclick = async () => {
    await api.clearLogs();
    renderView();
  };
  actions.append(btnCopy, btnExport, btnClear);
  head.append(actions);
  wrap.append(head);

  // level chips + search (#23)
  const filters = el("div", "srv-toolbar");
  for (const [k, label] of [
    ["", t("logs.allLevels")],
    ["error", t("logs.levelError")],
    ["warning", t("logs.levelWarning")],
    ["info", t("logs.levelInfo")],
  ] as [string, string][]) {
    const chip = el(
      "button",
      `chip filter-chip ${logLevel === k ? "on" : ""}`,
      label
    );
    chip.onclick = () => {
      logLevel = k;
      renderView();
    };
    filters.append(chip);
  }
  const search = el("input", "srv-search") as HTMLInputElement;
  search.placeholder = t("logs.search");
  search.value = logQuery;
  search.addEventListener("input", () => {
    logQuery = search.value;
    renderView();
    const again = document.querySelector(".srv-search") as HTMLInputElement | null;
    if (again) {
      again.focus();
      again.setSelectionRange(again.value.length, again.value.length);
    }
  });
  filters.append(search);
  wrap.append(filters);

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

  // crash history from the core monitor (#23) - only when there is any
  api.crashHistory().then((entries) => {
    if (!entries || entries.length === 0) return;
    const cCard = el("div", "card");
    cCard.append(el("h2", "", t("logs.crashes")));
    cCard.append(el("p", "muted", t("logs.crashesNote")));
    for (const e of entries.slice().reverse()) {
      const row = el("div", "switch-row");
      const when = new Date((e.time || 0) * 1000).toLocaleString();
      const tail = Array.isArray(e.tail) && e.tail.length ? e.tail[e.tail.length - 1] : "";
      row.append(
        el("span", "", when),
        el(
          "span",
          `badge ${e.code == null ? "fail" : "mid"}`,
          e.code == null ? t("logs.crash") : `code ${e.code}`
        ),
        el("span", "mono doctor-detail", String(tail).slice(0, 140))
      );
      cCard.append(row);
    }
    wrap.append(cCard);
  });

  const logBox = el("pre", "logbox", "");
  (logBox as any).id = "logbox";
  api.getLogs().then((lines) => {
    const q = logQuery.trim().toLowerCase();
    const filtered = lines.filter((l) => {
      if (logLevel && logLevelOf(l) !== logLevel) return false;
      if (q && !l.toLowerCase().includes(q)) return false;
      return true;
    });
    logBox.textContent = filtered.length
      ? filtered.join("\n")
      : lines.length
        ? t("logs.noMatches")
        : t("logs.empty");
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
// first-run wizard (#24)
// ---------------------------------------------------------------------------

/**
 * Four-step tour shown once on a fresh install (settings.json had no
 * wizard_done flag): language -> import -> geo data -> done. Skip or finish
 * records wizard_done so it never shows again.
 */
function firstRunWizard() {
  if (!settings || settings.wizard_done !== false) return;
  if (document.querySelector(".modal-back")) return;

  const dlg = el("div", "modal-back wizard");
  const box = el("div", "modal wizard-modal");
  dlg.append(box);
  document.body.append(dlg);

  let step = 0;
  let geoBusy = false;
  let geoDone = false;

  const finish = async () => {
    dlg.remove();
    try {
      await save({ wizard_done: true });
    } catch {
      // worst case: the wizard shows again on next launch
    }
  };

  const stepImport = () => {
    const w = el("div", "wizard-step");
    w.append(el("h2", "", t("wizard.importTitle")), el("p", "muted", t("wizard.importNote")));
    const ta = el("textarea") as HTMLTextAreaElement;
    ta.placeholder = t("servers.dialogAdd");
    w.append(ta);
    const row = el("div", "actions");
    const btnImport = el("button", "btn primary", t("servers.import"));
    btnImport.onclick = async () => {
      const data = ta.value.trim();
      if (!data) return;
      try {
        const n = await api.importLinks(data, "");
        servers = await api.listServers();
        await refreshData();
        w.append(el("p", "muted", t("wizard.imported").replace("{n}", String(n))));
        btnImport.disabled = true;
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
      }
    };
    const btnBrowse = el("button", "btn", t("wizard.browse"));
    btnBrowse.onclick = async () => {
      await finish();
      switchView("subs");
    };
    row.append(btnImport, btnBrowse);
    w.append(row);
    return w;
  };

  const stepGeo = () => {
    const w = el("div", "wizard-step");
    w.append(el("h2", "", t("wizard.geoTitle")), el("p", "muted", t("wizard.geoNote")));
    const btn = el(
      "button",
      "btn primary",
      geoBusy ? t("wizard.geoBusy") : geoDone ? t("wizard.geoDone") : t("routing.geoDownload")
    );
    btn.disabled = geoBusy || geoDone;
    btn.onclick = async () => {
      geoBusy = true;
      btn.disabled = true;
      btn.textContent = t("wizard.geoBusy");
      try {
        await api.downloadGeo();
        geoDone = true;
        btn.textContent = t("wizard.geoDone");
      } catch (e: any) {
        alert(`${t("error")}: ${e}`);
        btn.textContent = t("routing.geoDownload");
      } finally {
        geoBusy = false;
        btn.disabled = geoDone;
      }
    };
    w.append(btn);
    return w;
  };

  const steps: Array<() => HTMLElement> = [
    () => {
      const w = el("div", "wizard-step");
      w.append(el("h2", "", t("wizard.welcome")), el("p", "muted", t("wizard.langNote")));
      const row = el("div", "actions");
      const mk = (code: Lang, label: string) => {
        const b = el("button", `btn${getLang() === code ? " primary" : ""}`, label);
        b.onclick = () => {
          applyLang(code);
          void save({ language: code });
          draw();
        };
        row.append(b);
      };
      mk("en", "English");
      mk("fa", "فارسی");
      w.append(row);
      return w;
    },
    stepImport,
    stepGeo,
    () => {
      const w = el("div", "wizard-step");
      w.append(el("h2", "", t("wizard.doneTitle")), el("p", "muted", t("wizard.doneNote")));
      return w;
    },
  ];

  const draw = () => {
    box.textContent = "";
    box.append(steps[step]());
    const foot = el("div", "wizard-foot");
    const dots = el("div", "wizard-dots");
    steps.forEach((_, i) => dots.append(el("span", `dot${i === step ? " on" : ""}`)));
    const actions = el("div", "modal-actions");
    if (step > 0) {
      const back = el("button", "btn ghost", t("wizard.back"));
      back.onclick = () => {
        step--;
        draw();
      };
      actions.append(back);
    }
    const skip = el("button", "btn ghost", t("wizard.skip"));
    skip.onclick = () => void finish();
    actions.append(skip);
    const last = step >= steps.length - 1;
    const next = el("button", "btn primary", last ? t("wizard.finish") : t("wizard.next"));
    next.onclick = () => {
      if (last) void finish();
      else {
        step++;
        draw();
      }
    };
    actions.append(next);
    foot.append(dots, actions);
    box.append(foot);
  };
  draw();
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
  // #21: verify the full chain right after a connect attempt
  runDoctor(true);
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
    // #6: toast when ports had to shift to avoid a conflict
    listen("ports-shifted", (e: any) => {
      const changes = (e.payload?.changes || []) as [string, number, number][];
      if (changes.length) {
        toast(
          changes.map(([n, o, nw]) => `${n}: ${o} -> ${nw}`).join("   ")
        );
      }
    });
    // #23: the core died on its own - clear backend state and refresh UI
    listen("core-exited", async () => {
      try {
        await api.disconnect();
      } catch {
        /* state may already be gone */
      }
      updateStatus();
      if (currentView === "logs" || currentView === "connection") renderView();
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
  // #24: first-run wizard after first paint
  setTimeout(firstRunWizard, 600);
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

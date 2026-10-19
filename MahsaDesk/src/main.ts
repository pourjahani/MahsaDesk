import "./style.css";
import { api, type ServerProfile, type Subscription, type Settings } from "./api";
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

async function refreshData() {
  settings = await api.getSettings();
  servers = await api.listServers();
  subs = await api.listSubscriptions();
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
  { id: "settings", icon: "settings", label: "nav.settings" },
  { id: "logs", icon: "logs", label: "nav.logs" },
  { id: "about", icon: "about", label: "nav.about" },
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
  const btnStop = el("button", "btn ghost", "Stop");
  btnStop.id = "btn-stop";
  btnStop.style.display = "none";
  btnStop.onclick = () => doDisconnect();
  btnRow.append(btnSystem, btnTun, btnStop);
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

function renderView() {
  mainEl.innerHTML = "";
  switch (currentView) {
    case "servers": renderServers(); break;
    case "subs": renderSubs(); break;
    case "routing": renderRouting(); break;
    case "settings": renderSettings(); break;
    case "logs": renderLogs(); break;
    case "about": renderAbout(); break;
  }
}

async function updateStatus() {
  try {
    const s = await api.status();
    const running = s.running;
    statusDot.className = `dot ${running ? "on" : "off"}`;
    statusText.textContent = running
      ? `${t(s.mode === "tun" ? "status.tunMode" : "status.systemMode")} - ${t("status.connected")}`
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
  actions.append(btnImport, btnPing);
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
      await api.persist();
      servers = await api.listServers();
      renderView();
    };
    btnPing.append;
    right.append(btnPing, btnUse, btnEdit, btnDel);
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
  box.append(title, ta, row);
  dlg.append(box);
  document.body.append(dlg);
  cancel.onclick = () => dlg.remove();
  ok.onclick = async () => {
    const text = (ta.value || "").trim();
    if (!text) return;
    try {
      const n = await api.importLinks(text, "");
      dlg.remove();
      await api.persist();
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
  const ok = el("button", "btn primary", t("saved"));
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
    await api.persist();
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
  const ok = el("button", "btn primary", t("saved"));
  row.append(cancel, ok);
  box.append(title, form, row);
  dlg.append(box);
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
  for (const [k, v] of Object.entries(t === undefined ? {} : {})) { void k; void v; }
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
// settings view
// ---------------------------------------------------------------------------

function renderSettings() {
  const wrap = el("div", "view");
  const head = el("div", "view-head");
  head.append(el("h1", "", t("settings.title")));
  wrap.append(head);
  if (!settings) return;
  const s = settings;

  const mkSwitch = (title: string, key: keyof Settings) => {
    const row = el("div", "switch-row");
    row.append(el("span", "", title));
    const sw = el("button", `switch ${s[key] ? "on" : ""}`);
    sw.onclick = async () => {
      await save({ [key]: !s[key] } as any);
      (s as any)[key] = !s[key];
      sw.className = `switch ${s[key] ? "on" : ""}`;
    };
    row.append(sw);
    return row;
  };

  const mkInput = (title: string, key: keyof Settings, type = "text") => {
    const f = el("div", "field");
    f.append(el("label", "", title));
    const i = el("input") as HTMLInputElement;
    i.type = type;
    i.value = String(s[key] ?? "");
    i.addEventListener("change", () =>
      save({ [key]: type === "number" ? Number(i.value || 0) : i.value } as any)
    );
    f.append(i);
    return f;
  };

  const mkSelect = (title: string, key: keyof Settings, options: [string, string][]) => {
    const f = el("div", "field");
    f.append(el("label", "", title));
    const sel = el("select") as HTMLSelectElement;
    for (const [k, v] of options) {
      const o = el("option", "", v) as HTMLOptionElement;
      o.value = k;
      sel.append(o);
    }
    sel.value = String(s[key] ?? "");
    sel.onchange = () => save({ [key]: sel.value } as any);
    f.append(sel);
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
  general.append(mkSelect(t("settings.language"), "language", langOptions));
  general.append(
    mkSwitch(t("settings.minimizeTray"), "minimize_to_tray"),
    mkSwitch(t("settings.autoConnect"), "auto_connect")
  );
  wrap.append(general);

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

  wrap.append(el("p", "muted", t("settings.danger")));
  mainEl.append(wrap);
}

// ---------------------------------------------------------------------------
// logs view
// ---------------------------------------------------------------------------

let trafficUp = 0;
let trafficDown = 0;
let lastUp = 0;
let lastDown = 0;

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
        const dUp = up - lastUp;
        const dDown = down - lastDown;
        lastUp = lastUp === 0 ? 0 : lastUp;
        trafficEl.textContent = `- ${fmtBytes(down)} / ${fmtBytes(up)}`;
        void dUp;
        void dDown;
      }
      const upNum = document.querySelector(".traffic-num") as HTMLElement | null;
      if (upNum && currentView === "logs") {
        const els = document.querySelectorAll(".traffic-num");
        if (els[0]) els[0].textContent = fmtBytes(down);
        if (els[1]) els[1].textContent = fmtBytes(up);
      }
      lastUp = up;
      lastDown = down;
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
  buildLayout();
  renderView();
  updateStatus();
  listenTraffic();
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

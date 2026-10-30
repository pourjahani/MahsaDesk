import { invoke } from "@tauri-apps/api/core";
import { readImage } from "@tauri-apps/plugin-clipboard-manager";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ServerProfile {
  id: string;
  sub_id: string;
  remark: string;
  protocol: string;
  raw: string;
  address: string;
  port: number;
  auth: string;
  method: string;
  alter_id: number;
  encryption: string;
  net: string;
  header_type: string;
  host: string;
  path: string;
  security: string;
  sni: string;
  fingerprint: string;
  pbk: string;
  sid: string;
  spx: string;
  flow: string;
  alpn: string;
  delay: number | null;
}

export interface Subscription {
  id: string;
  name: string;
  url: string;
  enabled: boolean;
  update_interval_hours: number;
  last_update: number | null;
}

export interface Settings {
  language: string;
  theme: string;
  socks_port: number;
  http_port: number;
  loglevel: string;
  sniffing: boolean;
  allow_insecure: boolean;
  mux: boolean;
  ipv6: boolean;
  /** optional proxy used only for downloads; empty = automatic */
  download_proxy: string;
  remote_dns: string;
  routing_preset: string;
  custom_proxy: string;
  custom_direct: string;
  custom_block: string;
  minimize_to_tray: boolean;
  launch_at_login: boolean;
  auto_connect: boolean;
  active_server: string;
  fragment_enabled: boolean;
  fragment_packets: string;
  fragment_length: string;
  fragment_interval: string;
  cdn_mode: string;
  cdn_ip: string;
  domain_strategy: string;
  proxy_sharing: boolean;
  per_app_mode: string;
  per_app_list: string;
  psiphon_mode: string;
  mode: string;
  failover_mode: string;
  psiphon_socks_port: number;
  psiphon_http_port: number;
  psiphon_region: string;
  psiphon_protocol: string;
  psiphon_aggressive: boolean;
  ech_enabled: boolean;
  ech_config: string;
  front_enabled: boolean;
  front_host: string;
  share_port: number;
  share_lan: boolean;
  wizard_done: boolean | null;
}

export interface ShareInfo {
  url: string;
  local_url: string;
  lan_url: string | null;
  port: number;
  lan: boolean;
  /** Server ids this endpoint serves; null = the whole pool. */
  scope: string[] | null;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (s: Settings) => invoke<void>("set_settings", { newSettings: s }),
  listServers: () => invoke<ServerProfile[]>("list_servers"),
  saveServer: (p: ServerProfile) => invoke<void>("save_server", { dirServer: p }),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  deleteServers: (ids: string[]) => invoke<void>("delete_servers", { ids }),
  exportLinks: (ids: string[]) => invoke<string[]>("export_links", { ids }),
  shareStart: (port: number, lan: boolean, ids?: string[] | null) =>
    invoke<ShareInfo>("share_server_start", { port, lan, ids: ids ?? null }),
  shareStop: () => invoke<void>("share_server_stop"),
  shareStatus: () => invoke<ShareInfo | null>("share_server_status"),
  connectionDoctor: () => invoke<DoctorCheck[]>("connection_doctor"),
  crashHistory: () =>
    invoke<{ time: number; code: number | null; config: string; tail: string[] }[]>(
      "get_crash_history"
    ),
  exportLogs: (text: string) => invoke<string>("export_logs", { text }),
  ispPresets: () => invoke<IspPreset[]>("isp_presets"),
  detectIsp: (isp: string, org: string) =>
    invoke<string | null>("detect_isp", { isp, org }),
  setActive: (id: string) => invoke<void>("set_active", { id }),
  importLinks: (text: string, subId: string) =>
    invoke<number>("import_links", { text, subId: subId || null }),
  addSubscription: (name: string, url: string, hours: number, now: boolean) =>
    invoke<void>("add_subscription", { name, url, updateIntervalHours: hours, updateNow: now }),
  listSubscriptions: () => invoke<Subscription[]>("list_subscriptions"),
  editSubscription: (s: Subscription) => invoke<void>("edit_subscription", { sub: s }),
  deleteSubscription: (id: string) => invoke<void>("delete_subscription", { id }),
  updateSubscription: (id: string) => invoke<number>("update_subscription", { id }),
  testDelay: (id: string) => invoke<number | null>("test_delay", { id }),
  testAllDelays: (ids?: string[] | null) =>
    invoke<{ id: string; delay: number | null }[]>("test_all_delays", { ids: ids ?? null }),
  connect: (mode: string) => invoke<void>("connect", { mode }),
  disconnect: () => invoke<void>("disconnect_cmd"),
  status: () =>
    invoke<{ running: boolean; mode: string | null; hy2_running?: boolean }>("status"),
  getLogs: () => invoke<string[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  exportConfig: () => invoke<string>("export_config"),
  importConfig: (text: string) => invoke<void>("import_config", { jsonText: text }),
  downloadGeo: () => invoke<void>("download_geo"),
  geoExists: () => invoke<boolean>("geo_files_exist"),
  version: () => invoke<string>("app_version"),
  base64Encode: (t: string) => invoke<string>("base64_encode", { text: t }),
  shareLink: (id: string) => invoke<string>("share_link", { id }),
  runRealTest: (id: string) => invoke<number | null>("run_real_test", { id }),
  testAllReal: (ids?: string[] | null) =>
    invoke<{ id: string; delay: number | null }[]>("test_all_real", { ids: ids ?? null }),
  validateConfig: () => invoke<void>("validate_servers_config"),
  listProcesses: () => invoke<string[]>("list_running_processes"),
  assetsInfo: () =>
    invoke<{ name: string; exists: boolean; size: number }[]>("assets_info"),
  deleteGeo: (name: string) => invoke<void>("delete_geo", { name }),
  dnsScan: (domain: string) =>
    invoke<DnsResult[]>("dns_scan_cmd", { domain }),
  ipScan: (preset: string, custom: string, port: number) =>
    invoke<IpResult[]>("ip_scan_cmd", { preset, custom, port }),
  netTool: (kind: string, host: string) =>
    invoke<NetToolOutput>("net_tool_cmd", { kind, host }),
  portCheck: (host: string, port: number) =>
    invoke<PortCheckResult>("port_check_cmd", { host, port }),
  myIp: () => invoke<Record<string, string>>("my_ip_cmd"),
  netSpeed: (bytes: number) => invoke<number>("net_speed_cmd", { bytes }),
  requirementsStatus: (remote = true) =>
    invoke<ReqInfo[]>("requirements_status", { remote }),
  requirementsUpgrade: (key: string) =>
    invoke<ReqInfo>("requirements_upgrade", { key }),
  appUpdateCheck: () => invoke<AppUpdateInfo>("app_update_check"),
  appUpdateDownload: (url: string, version: string, sha256: string | null) =>
    invoke<string>("app_update_download", { url, version, sha256 }),
  appUpdateInstall: (path: string, portable: boolean) =>
    invoke<void>("app_update_install", { path, portable }),
  onAppUpdateProgress: (cb: (downloaded: number, total: number | null) => void) =>
    listen<{ downloaded: number; total: number | null }>("app-update-progress", (e) =>
      cb(e.payload.downloaded, e.payload.total)
    ),
};

export interface AppUpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  download_url: string;
  installer_url: string;
  installer_sha256: string;
  portable_url: string;
  portable_sha256: string;
  release_url: string;
  notes: string;
  /** true when the running build is the portable exe */
  portable: boolean;
}

export interface ReqInfo {
  key: string;
  title: string;
  exists: boolean;
  size: number;
  installed_version: string | null;
  available_version: string | null;
  update_available: boolean;
  /** file hash matches the pinned .sha marker (#25) */
  verified: boolean;
}

export interface IspPreset {
  id: string;
  fragment_enabled: boolean;
  packets: string;
  length: string;
  interval: string;
}

export interface DoctorCheck {
  id: string;
  ok: boolean;
  detail: string;
}

export interface DnsResult {
  server: string;
  kind: string;
  status: string;
  delay: number | null;
  ips: string[];
}

export interface IpResult {
  ip: string;
  ok: boolean;
  delay: number | null;
}

export interface NetToolOutput {
  ok: boolean;
  detail: string;
}

export interface PortCheckResult {
  open: boolean;
  delay: number | null;
}

/** Read a QR-coded config from an image sitting in the clipboard. */
export async function readQrFromClipboard(): Promise<string | null> {
  const { default: jsQR } = await import("jsqr");
  let image: any;
  try {
    image = await readImage();
  } catch {
    return null;
  }
  const rgba = (await image.toRgba?.()) ?? image.rgba;
  const w = image.width ?? 0;
  const h = image.height ?? 0;
  if (!rgba || !w || !h) return null;
  const code = jsQR(new Uint8ClampedArray(rgba), w, h);
  return code?.data ?? null;
}

export function onCoreLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<{ line: string }>("core-log", (e) => cb(e.payload.line));
}

export function onTraffic(cb: (up: number, down: number) => void): Promise<UnlistenFn> {
  return listen<{ up: number; down: number }>("traffic", (e) =>
    cb(e.payload.up, e.payload.down)
  );
}

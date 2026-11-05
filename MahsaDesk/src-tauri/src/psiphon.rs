use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};

use crate::core::push_log;
use crate::settings::Settings;

/// Name of the tunnel-core executable we expect in the binaries folder.
pub const EXE_NAME: &str = "psiphon-tunnel-core.exe";

/// Integer-only client version: tunnel-core runs `strconv.Atoi` on
/// `ClientVersion` while committing the config, so a semver like "1.0" makes
/// it reject the file and exit code 1 before the tunnel ever starts.
/// Dots stripped from the crate version ("0.3.6" -> "036").
pub fn client_version() -> String {
    env!("CARGO_PKG_VERSION").replace('.', "")
}

/// Default local ports (MahsaNG Android defaults).
pub const DEFAULT_SOCKS_PORT: u16 = 7501;
pub const DEFAULT_HTTP_PORT: u16 = 7502;

// ---------------------------------------------------------------------------
// Bootstrap network values
// ---------------------------------------------------------------------------
// The stock console-client engine embeds no network bootstrap. With the old
// all-F placeholders the engine booted with ZERO server candidates
// ("ServerEntryIterator.reset: entries 0", "tactics request aborted: no
// capable servers") and the untunneled discovery fetcher died on "no broker
// specs" - so the tunnel could never establish and every connect sat in its
// 90 s traffic probe before giving up.
//
// The values below are Psiphon's public remote/obfuscated server list
// endpoints + signature keys, the same bootstrap every community tunnel-core
// wrapper ships. With them a fresh engine downloads its server entries
// directly (S3 in ~1 s on an open network, mirrors after) and establishes.
// Verified end-to-end: local HTTP proxy answered generate_204 through the
// tunnel. Every field uses or_insert so a user/provider psiphon-user.json
// still overrides anything.

pub const SPONSOR_ID: &str = "1BC527D3D09985CF";
pub const PROPAGATION_CHANNEL_ID: &str = "92AACC5BABE0944C";

pub const SERVER_ENTRY_SIGNATURE_PUBLIC_KEY: &str = "sHuUVTWaRyh5pZwy4UguSgkwmBe0EHtJJkoF5WrxmvA=";

pub const REMOTE_SERVER_LIST_SIGNATURE_PUBLIC_KEY: &str = "MIICIDANBgkqhkiG9w0BAQEFAAOCAg0AMIICCAKCAgEAt7Ls+/39r+T6zNW7GiVpJfzq/xvL9SBH5rIFnk0RXYEYavax3WS6HOD35eTAqn8AniOwiH+DOkvgSKF2caqk/y1dfq47Pdymtwzp9ikpB1C5OfAysXzBiwVJlCdajBKvBZDerV1cMvRzCKvKwRmvDmHgphQQ7WfXIGbRbmmk6opMBh3roE42KcotLFtqp0RRwLtcBRNtCdsrVsjiI1Lqz/lH+T61sGjSjQ3CHMuZYSQJZo/KrvzgQXpkaCTdbObxHqb6/+i1qaVOfEsvjoiyzTxJADvSytVtcTjijhPEV6XskJVHE1Zgl+7rATr/pDQkw6DPCNBS1+Y6fy7GstZALQXwEDN/qhQI9kWkHijT8ns+i1vGg00Mk/6J75arLhqcodWsdeG/M/moWgqQAnlZAGVtJI1OgeF5fsPpXu4kctOfuZlGjVZXQNW34aOzm8r8S0eVZitPlbhcPiR4gT/aSMz/wd8lZlzZYsje/Jr8u/YtlwjjreZrGRmG8KMOzukV3lLmMppXFMvl4bxv6YFEmIuTsOhbLTwFgh7KYNjodLj/LsqRVfwz31PgWQFTEPICV7GCvgVlPRxnofqKSjgTWI4mxDhBpVcATvaoBl1L/6WLbFvBsoAUBItWwctO2xalKxF5szhGm8lccoc5MZr8kfE0uxMgsxz4er68iCID+rsCAQM=";

/// obfuscated base64 of server-list/OSL transfer URLs: the engine decodes
/// the "URL" field itself. Order: primary S3 (verified, no skip), then
/// domain-fronted mirrors that only run after the earlier ones fail.
const REMOTE_SERVER_LIST_URLS: &str =
    "[{\"OnlyAfterAttempts\":0,\"SkipVerify\":false,\"URL\":\"aHR0cHM6Ly9zMy5hbWF6b25hd3MuY29tL3BzaXBob24vd2ViL21qcjQtcDIzci1wdXdsL3NlcnZlcl9saXN0X2NvbXByZXNzZWQ=\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cubGF0aW5vZmlybWRkaG9zdHMuY29tL3dlYi9tanI0LXAyM3ItcHV3bC9zZXJ2ZXJfbGlzdF9jb21wcmVzc2Vk\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cuY29ycG9yYXRlaGlyZXByZXNzdGguY29tL3dlYi9tanI0LXAyM3ItcHV3bC9zZXJ2ZXJfbGlzdF9jb21wcmVzc2Vk\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cuZ3BhbGx0aGluZ3NudW1iZXJ3ZWF0aGVyLmNvbS93ZWIvbWpyNC1wMjNyLXB1d2wvc2VydmVyX2xpc3RfY29tcHJlc3NlZA==\"}]";
const OSL_ROOT_URLS: &str =
    "[{\"OnlyAfterAttempts\":0,\"SkipVerify\":false,\"URL\":\"aHR0cHM6Ly9zMy5hbWF6b25hd3MuY29tL3BzaXBob24vd2ViL21qcjQtcDIzci1wdXdsL29zbA==\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cubGF0aW5vZmlybWRkaG9zdHMuY29tL3dlYi9tanI0LXAyM3ItcHV3bC9vc2w=\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cuY29ycG9yYXRlaGlyZXByZXNzdGguY29tL3dlYi9tanI0LXAyM3ItcHV3bC9vc2w=\"},\n{\"OnlyAfterAttempts\":2,\"SkipVerify\":true,\"URL\":\"aHR0cHM6Ly93d3cuZ3BhbGx0aGluZ3NudW1iZXJ3ZWF0aGVyLmNvbS93ZWIvbWpyNC1wMjNyLXB1d2wvb3Ns\"}]";

/// Build the psiphon-tunnel-core config JSON.
///
/// Follows MahsaNG's PsiphonVpnService.getPsiphonConfig():
/// - optional user-supplied base config `psiphon_config.json` (from the app data dir)
/// - `UpstreamProxyURL` for "psiphon after" mode
/// - `EgressRegion`, `AggressiveEstablishment`, `LimitTunnelProtocols`
/// - fixed placeholder propagation/sponsor ids (same as MahsaNG)
/// - diagnostic emitter keys so the logs page shows tunnel status
pub fn build_config(
    settings: &Settings,
    upstream_socks_port: Option<u16>,
    data_dir: &Path,
    base_config_path: &Path,
) -> Value {
    let mut cfg = match std::fs::read_to_string(base_config_path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };

    if let Value::Object(ref mut map) = cfg {
        if let Some(socks) = upstream_socks_port {
            map.insert(
                "UpstreamProxyURL".into(),
                json!(format!("socks5://127.0.0.1:{}", socks)),
            );
        }
        if !settings.psiphon_region.trim().is_empty() {
            map.insert("EgressRegion".into(), json!(settings.psiphon_region.trim().to_uppercase()));
        }
        if settings.psiphon_aggressive {
            map.insert("AggressiveEstablishment".into(), json!(true));
        }
        match settings.psiphon_protocol.as_str() {
            "normal" => {
                map.insert(
                    "LimitTunnelProtocols".into(),
                    json!([
                        "SSH",
                        "OSSH",
                        "TLS-OSSH",
                        "QUIC-OSSH",
                        "SHADOWSOCKS-OSSH"
                    ]),
                );
            }
            "conduit" => {
                map.insert(
                    "LimitTunnelProtocols".into(),
                    json!([
                        "INPROXY-WEBRTC-OSSH",
                        "INPROXY-WEBRTC-UNFRONTED-MEEK-HTTPS-OSSH",
                        "INPROXY-WEBRTC-UNFRONTED-MEEK-SESSION-TICKET-OSSH",
                        "INPROXY-WEBRTC-FRONTED-MEEK-OSSH",
                        "INPROXY-WEBRTC-FRONTED-MEEK-HTTP-OSSH",
                        "INPROXY-WEBRTC-QUIC-OSSH"
                    ]),
                );
            }
            _ => {}
        }
        map.insert("LocalSocksProxyPort".into(), json!(settings.psiphon_socks_port));
        map.insert("LocalHttpProxyPort".into(), json!(settings.psiphon_http_port));

        // Bootstrap values above are documented with their constants. Note
        // the remote server list URLs/keys are inserted only when absent:
        // a base config from the user's own provider must keep its own list.
        map.entry("PropagationChannelId")
            .or_insert_with(|| json!(PROPAGATION_CHANNEL_ID));
        map.entry("SponsorId").or_insert_with(|| json!(SPONSOR_ID));
        map.entry("ServerEntrySignaturePublicKey")
            .or_insert_with(|| json!(SERVER_ENTRY_SIGNATURE_PUBLIC_KEY));
        map.entry("RemoteServerListSignaturePublicKey")
            .or_insert_with(|| json!(REMOTE_SERVER_LIST_SIGNATURE_PUBLIC_KEY));
        map.entry("UseIndistinguishableTLS")
            .or_insert_with(|| json!(true));
        if !map.contains_key("RemoteServerListURLs") {
            map.insert(
                "RemoteServerListURLs".into(),
                serde_json::from_str(REMOTE_SERVER_LIST_URLS).unwrap_or(json!([])),
            );
        }
        if !map.contains_key("ObfuscatedServerListRootURLs") {
            map.insert(
                "ObfuscatedServerListRootURLs".into(),
                serde_json::from_str(OSL_ROOT_URLS).unwrap_or(json!([])),
            );
        }
        map.insert("ClientPlatform".into(), json!("Windows"));
        map.insert("ClientVersion".into(), json!(client_version()));
        map.insert("EmitDiagnosticNotices".into(), json!(true));
        map.insert("EmitDiagnosticNetworkParameters".into(), json!(true));
        map.insert("EmitBytesTransferred".into(), json!(true));
        map.insert("EmitServerAlerts".into(), json!(true));
        map.insert(
            "DNSResolverAlternateServers".into(),
            json!(["1.1.1.1", "1.0.0.1", "8.8.8.8", "8.8.4.4"]),
        );
        map.entry("EstablishTunnelTimeoutSeconds")
            .or_insert_with(|| json!(0));
        map.insert(
            "DataRootDirectory".into(),
            json!(data_dir.to_string_lossy().into_owned()),
        );
    }
    cfg
}

/// Spawn the tunnel-core child with the given config; pipe its stdout/stderr
/// into the shared log buffer. Returns the child on success.
pub fn spawn_tunnel(
    exe_path: &PathBuf,
    workdir: &PathBuf,
    cfg: &Value,
    logs: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>,
    area: &tauri::AppHandle,
) -> Result<Child, String> {
    let cfg_path = workdir.join("psiphon-config.json");
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, text).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(exe_path);
    cmd.arg("-config")
        .arg(&cfg_path)
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::proc::hide(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start psiphon-tunnel-core: {}", e))?;
    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let logs = logs.clone();
        let area = area.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::<String, _>::ok) {
                push_log(&logs, &area, format!("[psiphon] {}", line));
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let logs = logs.clone();
        let area = area.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::<String, _>::ok) {
                push_log(&logs, &area, format!("[psiphon] {}", line));
            }
        });
    }
    push_log(logs, area, format!("[psiphon] tunnel-core started, pid={}", pid));

    // fail fast if the binary exits immediately (poll instead of a blind sleep)
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    let mut early = None;
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => {
                early = Some(status);
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(_) => break,
        }
    }
    if let Some(status) = early {
        return Err(format!(
            "psiphon-tunnel-core exited immediately: {} (check that the exe is valid; see logs)",
            status
        ));
    }
    Ok(child)
}

/// Block until the Psiphon SOCKS port accepts connections (max ~20 s), or
/// return an error so the caller can roll back cleanly.
pub fn wait_for_socks(socks_port: u16) -> Result<(), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", socks_port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err("Psiphon tunnel did not open its SOCKS port in time (still connecting?)".into())
}
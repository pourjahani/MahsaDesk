//! Requirement discovery, status checks and in-app upgrades.
//!
//! The app ships every runtime requirement inside the build (xray, sing-box,
//! wintun, geo data, the Psiphon engine). This module lets the UI verify what
//! is present, report versions, and pull newer copies.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// MahsaNG's Xray fork (same core family the app configs assume).
const XRAY_REPO: &str = "GFW-knocker/Xray-core";
/// sing-box pinned to 1.10.x: classic config format the TUN builder emits.
const SINGBOX_PINNED: &str = "1.10.7";
/// Official Psiphon release-candidate engine binaries (rolling branch).
const PSIPHON_BASE_URL: &str =
    "https://raw.githubusercontent.com/Psiphon-Labs/psiphon-tunnel-core-binaries/master/windows/psiphon-tunnel-core-i686.exe";
const PSIPHON_SHA_URL: &str =
    "https://api.github.com/repos/Psiphon-Labs/psiphon-tunnel-core-binaries/contents/windows/psiphon-tunnel-core-i686.exe";
/// wintun.dll: the sing-box release archive stopped shipping it (sing-box now
/// embeds its own copy via `memmod`), so the card's old source always failed
/// with "member matching pattern not found in <temp zip>". Pull the exact file
/// sing-tun keeps in-tree - the one shipped sing-box.exe loads - pinned to a
/// commit and checked by git blob sha, like the Psiphon engine.
const WINTUN_URL: &str = "https://raw.githubusercontent.com/SagerNet/sing-tun/611d83b7a57b4719be45581eb674942ff561de6c/internal/wintun/amd64/wintun.dll";
const WINTUN_BLOB_SHA: &str = "aee04e77bfbd6601adbcd5a46435f059e31cf6e4";
/// This app's own release feed: startup update check + download link.
const APP_REPO: &str = "pourjahani/MahsaDesk";

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("MahsaDesk")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[derive(Serialize, Clone)]
pub struct ReqInfo {
    pub key: String,
    pub title: String,
    pub exists: bool,
    pub size: u64,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub update_available: bool,
    /// File hash matches the pinned `.sha` marker written at install time.
    pub verified: bool,
}

fn exe_version(path: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(path);
    cmd.args(args);
    let out = crate::proc::output(cmd).ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr);
    Some(text.lines().next().unwrap_or("").trim().to_string())
    .filter(|s| !s.is_empty())
}

fn file_probe(dir: &Path, name: &str) -> Option<(u64, Option<String>)> {
    let p = dir.join(name);
    if !p.exists() {
        return None;
    }
    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    let version = match name {
        "xray.exe" => exe_version(&p, &["version"]),
        "sing-box.exe" => exe_version(&p, &["version"]),
        "psiphon-tunnel-core.exe" => exe_version(&p, &["--version"]),
        _ => None,
    };
    Some((size, version))
}

/// Query the GitHub tag of the latest release of `repo` (same helper the
/// build script uses).
pub async fn github_latest_tag(repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let v: serde_json::Value = http().get(&url).send().await.ok()?.json().await.ok()?;
    v.get("tag_name")
        .and_then(|t| t.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
}

/// Blob sha of the current Psiphon engine binary (rolling master branch).
pub async fn psiphon_engine_sha() -> Option<String> {
    let v: serde_json::Value = http()
        .get(PSIPHON_SHA_URL)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    v.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string())
}

fn sha_marker(dir: &Path) -> PathBuf {
    dir.join("psiphon-tunnel-core.sha")
}

// ---------------------------------------------------------------------------
// Integrity verification (#25): pinned hash markers + GitHub asset digests
// ---------------------------------------------------------------------------

pub fn sha256_bytes(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn sha256_file(path: &Path) -> Option<String> {
    use sha2::Digest;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}

/// Git blob object id of a file: sha1("blob <len>\\0" + bytes) — the same
/// hash the GitHub contents API reports, used for the rolling Psiphon engine.
pub fn git_blob_sha1(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(format!("blob {}\0", data.len()).as_bytes());
    h.update(data);
    format!("{:x}", h.finalize())
}

fn marker_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_owned();
    s.push(".sha");
    PathBuf::from(s)
}

fn write_marker(file: &Path, sha256_hex: &str) {
    let _ = std::fs::write(
        marker_path(file),
        format!("sha256:{}", sha256_hex),
    );
}

/// Write the verification marker for a geo data file after downloading it.
pub fn write_geo_marker(file: &Path, bytes: &[u8]) {
    write_marker(file, &sha256_bytes(bytes));
}

/// True when the file exists and its sha256 matches its pinned marker.
fn marker_verified(dir: &Path, name: &str) -> bool {
    let file = dir.join(name);
    let Ok(marker) = std::fs::read_to_string(marker_path(&file)) else {
        return false;
    };
    match marker.trim().strip_prefix("sha256:") {
        Some(expected) => sha256_file(&file).map(|h| h == expected).unwrap_or(false),
        None => false,
    }
}

/// Psiphon markers predate the sha256 scheme: they hold the git blob sha.
fn psiphon_marker_verified(dir: &Path) -> bool {
    let Ok(marker) = std::fs::read_to_string(sha_marker(dir)) else {
        return false;
    };
    let expected = marker.trim();
    if expected.is_empty() {
        return false;
    }
    std::fs::read(dir.join(crate::psiphon::EXE_NAME))
        .map(|b| git_blob_sha1(&b) == expected)
        .unwrap_or(false)
}

/// Installed vs available status for one requirement.
pub async fn check_requirement(dir: &PathBuf, key: &str, remote: bool) -> ReqInfo {
    let mut info = match key {
        "xray" => {
            let (exists, size, ver) = match file_probe(dir, "xray.exe") {
                Some((size, v)) => (true, size, v),
                None => (false, 0, None),
            };
            let mut info = ReqInfo {
                key: key.into(),
                title: "Xray core".into(),
                exists,
                size,
                installed_version: ver,
                available_version: None,
                update_available: false,
                verified: false,
            };
            if remote {
                let tag = github_latest_tag(XRAY_REPO).await;
                info.available_version = tag.clone();
                info.update_available = matches!(
                    (&info.installed_version, &tag),
                    (Some(inst), Some(avail)) if !inst.contains(avail.as_str())
                );
            }
            info
        }
        "singbox" => {
            let (exists, size, ver) = match file_probe(dir, "sing-box.exe") {
                Some((size, v)) => (true, size, v),
                None => (false, 0, None),
            };
            let mut info = ReqInfo {
                key: key.into(),
                title: "sing-box (TUN engine)".into(),
                exists,
                size,
                installed_version: ver,
                available_version: Some(SINGBOX_PINNED.into()),
                update_available: false,
                verified: false,
            };
            // pinned channel: flag it only when the build target differs
            if !exists
                || info
                    .installed_version
                    .as_deref()
                    .map(|v| !v.contains(SINGBOX_PINNED))
                    .unwrap_or(true)
            {
                info.update_available = true;
            }
            info
        }
        "wintun" => {
            let (exists, size) = match file_probe(dir, "wintun.dll") {
                Some((size, _)) => (true, size),
                None => (false, 0),
            };
            ReqInfo {
                key: key.into(),
                title: "wintun.dll (TUN driver)".into(),
                exists,
                size,
                installed_version: None,
                available_version: Some("0.14.1".into()),
                update_available: !exists,
                verified: false,
            }
        }
        "geo" => {
            let a = dir.join("geoip.dat");
            let b = dir.join("geosite.dat");
            let exists = a.exists() || b.exists();
            let size = a
                .metadata()
                .map(|m| m.len())
                .unwrap_or(0)
                + b.metadata().map(|m| m.len()).unwrap_or(0);
            ReqInfo {
                key: key.into(),
                title: "Geo data (geoip/geosite)".into(),
                exists,
                size,
                installed_version: None,
                available_version: None,
                update_available: true, // refreshing data is always an option
                verified: false,
            }
        }
        "psiphon" => {
            let (exists, size, ver) = match file_probe(dir, "psiphon-tunnel-core.exe") {
                Some((size, v)) => (true, size, v),
                None => (false, 0, None),
            };
            let installed_sha = std::fs::read_to_string(sha_marker(dir)).ok();
            let mut info = ReqInfo {
                key: key.into(),
                title: "Psiphon tunnel engine".into(),
                exists,
                size,
                installed_version: None,
                available_version: None,
                update_available: false,
                verified: false,
            };
            if exists {
                // tolerant wrapper: report the probed version if there is one
                info.installed_version = ver;
            }
            if remote {
                let sha = psiphon_engine_sha().await;
                info.available_version = sha.clone().map(|_| "latest (rolling)".into());
                info.update_available = match (installed_sha, sha) {
                    (Some(cur), Some(new)) => cur.trim() != new.trim(),
                    (Some(_), None) => false,
                    (None, Some(_)) => true, // no sha marker yet -> offer refresh
                    (None, None) => false,
                };
            }
            info
        }
        _ => ReqInfo {
            key: key.into(),
            title: key.into(),
            exists: false,
            size: 0,
            installed_version: None,
            available_version: None,
            update_available: false,
            verified: false,
        },
    };
    if !info.exists {
        info.update_available = true;
    }
    // #25: green badge only when the on-disk hash matches the pinned marker
    info.verified = info.exists
        && match key {
            "xray" => marker_verified(dir, "xray.exe"),
            "singbox" => marker_verified(dir, "sing-box.exe"),
            "wintun" => marker_verified(dir, "wintun.dll"),
            "geo" => marker_verified(dir, "geoip.dat") && marker_verified(dir, "geosite.dat"),
            "psiphon" => psiphon_marker_verified(dir),
            _ => false,
        };
    info
}

pub async fn status_all(dir: &PathBuf, remote: bool) -> Vec<ReqInfo> {
    let mut out = Vec::new();
    for key in ["xray", "singbox", "wintun", "geo", "psiphon"] {
        out.push(check_requirement(dir, key, remote).await);
    }
    out
}

// ---------------------------------------------------------------------------
// Download / upgrade
// ---------------------------------------------------------------------------

fn unzip_member(zip_path: &Path, wanted: &dyn Fn(&str) -> bool, dst: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_string();
        if !wanted(&name) || f.is_dir() {
            continue;
        }
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        std::fs::write(dst, &buf).map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err(format!("member matching pattern not found in {}", zip_path.display()))
}

// ---------------------------------------------------------------------------
// Integrity verification (#25)
// ---------------------------------------------------------------------------

/// GitHub release asset digest ("sha256:..." published by the API since
/// 2025) for one asset of a release — `tag` None means "latest".
pub async fn github_asset_digest(
    repo: &str,
    tag: Option<&str>,
    asset_name: &str,
) -> Option<String> {
    let url = match tag {
        Some(t) => format!("https://api.github.com/repos/{}/releases/tags/{}", repo, t),
        None => format!("https://api.github.com/repos/{}/releases/latest", repo),
    };
    let v: serde_json::Value = http().get(&url).send().await.ok()?.json().await.ok()?;
    let assets = v.get("assets")?.as_array()?;
    let digest = assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(asset_name))?
        .get("digest")
        .and_then(|d| d.as_str())?;
    if digest.is_empty() {
        return None;
    }
    Some(digest.trim_start_matches("sha256:").to_lowercase())
}

/// Download `url`; when the publisher exposes an expected sha256, refuse
/// anything that doesn't match it. Returns the raw bytes either way.
pub async fn fetch_verified(url: &str, expected_sha256: Option<&str>) -> Result<Vec<u8>, String> {
    let resp = http().get(url).send().await.map_err(|e| {
        format!(
            "network error downloading from {}: {}. Check internet access or the Windows system proxy, then retry.",
            url, e
        )
    })?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} from {}", resp.status(), url));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let actual = sha256_bytes(&bytes);
    if let Some(expected) = expected_sha256 {
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(format!(
                "integrity check failed for {}: expected sha256 {}, got {}",
                url, expected, actual
            ));
        }
    }
    Ok(bytes.to_vec())
}

/// Split a GitHub release download URL into (repo, tag, asset name) so we can
/// ask the API for that exact asset's digest.
fn gh_asset_locus(zip_url: &str) -> Option<(String, Option<String>, String)> {
    let rest = zip_url.strip_prefix("https://github.com/")?;
    let (repo, tail) = rest.split_once("/releases/")?;
    let asset = zip_url.rsplit('/').next()?.to_string();
    let tag = tail
        .strip_prefix("download/")
        .and_then(|t| t.split('/').next())
        .map(|t| t.to_string());
    Some((repo.to_string(), tag, asset))
}

struct ZipFetch {
    zip_member: String,
    zip_url: String,
}

fn source_for(key: &str) -> Result<Option<ZipFetch>, String> {
    Ok(match key {
        "xray" => Some(ZipFetch {
            zip_url: format!(
                "https://github.com/{}/releases/latest/download/Xray-windows-64.zip",
                XRAY_REPO
            ),
            zip_member: "xray.exe".into(),
        }),
        "singbox" => Some(ZipFetch {
            zip_url: format!(
                "https://github.com/SagerNet/sing-box/releases/download/v{}/sing-box-{}-windows-amd64.zip",
                SINGBOX_PINNED, SINGBOX_PINNED
            ),
            zip_member: "sing-box.exe".into(),
        }),
        // not a zip: fetched and verified directly in upgrade()
        "wintun" => None,
        "geo" => None, // handled by the existing download_geo command
        _ => return Ok(None),
    })
}

/// Download and install one requirement into `dir`. Geo data goes through the
/// existing geo downloader; the Psiphon engine is a raw exe download plus a
/// sha marker so future upgrades can detect changes.
pub async fn upgrade(dir: &PathBuf, key: &str) -> Result<ReqInfo, String> {
    match key {
        // Psiphon publishes the git blob sha of the rolling engine binary:
        // downloaded bytes must hash to exactly that before we keep them.
        "psiphon" => {
            let dst = dir.join(crate::psiphon::EXE_NAME);
            let bytes = fetch_verified(PSIPHON_BASE_URL, None).await?;
            let local_sha = git_blob_sha1(&bytes);
            if let Some(remote) = psiphon_engine_sha().await {
                if remote.trim() != local_sha {
                    return Err(format!(
                        "Psiphon integrity check failed: published {} != downloaded {}",
                        remote.trim(),
                        local_sha
                    ));
                }
            }
            std::fs::write(&dst, &bytes).map_err(|e| e.to_string())?;
            let _ = std::fs::write(sha_marker(dir), &local_sha);
        }
        "geo" => {
            for (fname, url) in [
                (
                    "geoip.dat",
                    "https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat",
                ),
                (
                    "geosite.dat",
                    "https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat",
                ),
            ] {
                let expected =
                    github_asset_digest("Loyalsoldier/v2ray-rules-dat", None, fname).await;
                let bytes = fetch_verified(url, expected.as_deref()).await?;
                let dst = dir.join(fname);
                let tmp = dir.join(format!("{}.download", fname));
                std::fs::write(&tmp, &bytes)
                    .map_err(|e| format!("failed to save {}: {}", fname, e))?;
                // Windows rename does not replace an existing destination.
                // Remove the old file only after the new download is complete.
                if dst.exists() {
                    std::fs::remove_file(&dst)
                        .map_err(|e| format!("failed to replace {}: {}", fname, e))?;
                }
                if let Err(e) = std::fs::rename(&tmp, &dst) {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(format!("failed to install {}: {}", fname, e));
                }
                write_geo_marker(&dst, &bytes);
            }
        }
        "wintun" => {
            // Raw file, not an archive member - pin it by git blob sha so a
            // swapped-in DLL can never reach the binaries folder.
            let bytes = fetch_verified(WINTUN_URL, None).await?;
            let local = git_blob_sha1(&bytes);
            if local != WINTUN_BLOB_SHA {
                return Err(format!(
                    "wintun integrity check failed: published {} != downloaded {}",
                    WINTUN_BLOB_SHA, local
                ));
            }
            let dst = dir.join("wintun.dll");
            std::fs::write(&dst, &bytes).map_err(|e| e.to_string())?;
            write_marker(&dst, &sha256_bytes(&bytes));
        }
        other => {
            let Some(ZipFetch { zip_url, zip_member }) = source_for(other)? else {
                return Err(format!("no download source for {}", other));
            };
            // verify the archive against the release asset digest when GitHub
            // publishes one, then pin the extracted file's own hash
            let expected = match gh_asset_locus(&zip_url) {
                Some((repo, tag, asset)) => {
                    github_asset_digest(&repo, tag.as_deref(), &asset).await
                }
                None => None,
            };
            let bytes = fetch_verified(&zip_url, expected.as_deref()).await?;
            let tmp = std::env::temp_dir().join(format!("mahsareq-{}-{}.zip", other, std::process::id()));
            std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
            let dst = dir.join(zip_member.as_str());
            unzip_member(&tmp, &|n: &str| {
                n.ends_with(&zip_member) || n == zip_member
            }, &dst)?;
            if let Some(hash) = sha256_file(&dst) {
                write_marker(&dst, &hash);
            }
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(check_requirement(dir, key, false).await)
}

// ---------------------------------------------------------------------------
// Self-update check (same pattern as the requirements card)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct AppUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    /// Direct installer asset URL, or "" when the release has none.
    pub download_url: String,
    /// Human release / download page.
    pub release_url: String,
}

/// True when semver-ish `a` is strictly newer than `b` (numeric part compare,
/// string fallback for odd tags like "1.2.3-rc.1").
fn is_newer(a: &str, b: &str) -> bool {
    let nums = |s: &str| -> Vec<(u64, String)> {
        s.trim_start_matches('v')
            .split(|c| c == '.' || c == '-')
            .map(|p| (
                p.parse::<u64>().unwrap_or(0),
                p.to_string(),
            ))
            .collect()
    };
    let (a, b) = (nums(a), nums(b));
    for i in 0..a.len().max(b.len()) {
        let av = a.get(i);
        let bv = b.get(i);
        match (av, bv) {
            (Some(x), Some(y)) => {
                if x.0 != y.0 {
                    return x.0 > y.0;
                }
                if x.1 != y.1 {
                    // suffix compare: a numeric part beats a textual one
                    // (1.2 > 1.2-rc); textual vs textual is alphabetical best-effort
                    let x_num = x.1.parse::<u64>().is_ok();
                    let y_num = y.1.parse::<u64>().is_ok();
                    return match (x_num, y_num) {
                        (true, false) => true,
                        (false, true) => false,
                        _ => *x.1 > *y.1,
                    };
                }
            }
            // a plain release tag (numeric extras) beats a shorter prefix,
            // but a textual extra like "-rc.1" is a prerelease -> older
            (Some(x), None) => return x.1.parse::<u64>().is_ok(),
            // a textual extra on the other side means THAT one is the prerelease
            (None, Some(y)) => return !y.1.parse::<u64>().is_ok(),
            (None, None) => {}
        }
    }
    false
}

/// Check GitHub for a newer MahsaDesk release than the running build and
/// resolve a direct installer download URL from the release assets.
pub async fn check_app_update(current: &str) -> AppUpdateInfo {
    let current = current.trim().trim_start_matches('v').to_string();
    let release_url = format!("https://github.com/{}/releases/latest", APP_REPO);
    let url = format!("https://api.github.com/repos/{}/releases/latest", APP_REPO);
    let fallback = AppUpdateInfo {
        current_version: current.clone(),
        latest_version: current.clone(),
        update_available: false,
        download_url: String::new(),
        release_url: release_url.clone(),
    };
    let rel: serde_json::Value = match http().get(&url).send().await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(_) => return fallback,
        },
        Err(_) => return fallback,
    };
    let tag = rel
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if tag.is_empty() {
        return fallback;
    }
    // pick the NSIS installer first, then any windows .exe asset
    let mut download_url = String::new();
    if let Some(assets) = rel.get("assets").and_then(|a| a.as_array()) {
        let pick = |pred: &dyn Fn(&str) -> bool| -> Option<String> {
            assets
                .iter()
                .filter_map(|a| {
                    let name = a.get("name").and_then(|n| n.as_str())?;
                    let url = a.get("browser_download_url").and_then(|u| u.as_str())?;
                    Some((name, url))
                })
                .find(|(n, _)| pred(n))
                .map(|(_, u)| u.to_string())
        };
        download_url = pick(&|n| {
            n.contains("setup") && n.ends_with(".exe")
        })
            .or_else(|| pick(&|n| n.contains("x64") && n.ends_with(".exe")))
            .or_else(|| {
                pick(&|n| {
                    n.ends_with(".exe")
                        && !n.contains("portable")
                        && !n.contains("msi")
                })
            })
            .unwrap_or_default();
    }
    let url_boxed = download_url.clone();
    let release_page = rel
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or(&release_url)
        .to_string();
    AppUpdateInfo {
        update_available: is_newer(&tag, &current),
        latest_version: tag,
        download_url: url_boxed,
        release_url: release_page,
        ..fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_patches_and_minors() {
        assert!(is_newer("0.3.2", "0.3.1"));
        assert!(is_newer("0.4.0", "0.3.9"));
        assert!(is_newer("1.0.0", "0.9.99"));
    }

    #[test]
    fn same_and_older_not_flagged() {
        assert!(!is_newer("0.3.1", "0.3.1"));
        assert!(!is_newer("0.3.0", "0.3.1"));
    }

    #[test]
    fn v_prefix_and_dangling_parts() {
        assert!(is_newer("v0.4.0", "0.3.1"));
        assert!(is_newer("0.4", "0.3"));
        assert!(!is_newer("0.3", "0.3.1"));
    }

    #[test]
    fn odd_suffixes_fall_back_to_string_compare() {
        assert!(is_newer("0.3.2", "0.3.1-beta"));
        assert!(!is_newer("0.3.1-alpha", "0.3.1"));
    }

    #[test]
    fn release_beats_prerelease_of_same_version() {
        assert!(is_newer("0.3.1", "0.3.1-rc2"));
        assert!(!is_newer("0.3.1-rc1", "0.3.1"));
    }

    #[test]
    fn sha256_of_empty_is_known_vector() {
        assert_eq!(
            sha256_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn git_blob_sha1_matches_git_hash_object() {
        // $ echo -n $'hello\n' | git hash-object --stdin
        assert_eq!(git_blob_sha1(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn wintun_is_never_sourced_from_the_sing_box_archive() {
        // The sing-box windows archive contains only LICENSE and sing-box.exe,
        // so a zip fetch for wintun.dll always ended in
        // "member matching pattern not found in C:\...\mahsareq-wintun-*.zip".
        assert!(source_for("wintun").unwrap().is_none());
    }

    #[test]
    fn gh_asset_locus_parses_release_urls() {
        let (repo, tag, asset) = gh_asset_locus(
            "https://github.com/SagerNet/sing-box/releases/download/v1.10.7/sing-box-1.10.7-windows-amd64.zip",
        )
        .unwrap();
        assert_eq!(repo, "SagerNet/sing-box");
        assert_eq!(tag.as_deref(), Some("v1.10.7"));
        assert_eq!(asset, "sing-box-1.10.7-windows-amd64.zip");

        let (repo, tag, asset) = gh_asset_locus(
            "https://github.com/GFW-knocker/Xray-core/releases/latest/download/Xray-windows-64.zip",
        )
        .unwrap();
        assert_eq!(repo, "GFW-knocker/Xray-core");
        assert_eq!(tag, None);
        assert_eq!(asset, "Xray-windows-64.zip");
    }
}

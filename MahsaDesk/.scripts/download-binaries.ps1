param(
  [string]$OutDir = "$PSScriptRoot/../src-tauri/binaries"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Github-LatestAssetZip([string]$Repo, [string]$Match) {
  $rel = Invoke-RestMethod -Headers @{ "User-Agent" = "MahsaDesk" } -Uri "https://api.github.com/repos/$Repo/releases/latest"
  $asset = $rel.assets | Where-Object { $_.name -match $Match } | Select-Object -First 1
  if (-not $asset) { throw "no asset matching $Match in $Repo latest release" }
  return $asset.browser_download_url
}

function Expand-ZipTo([string]$Url, [string]$Dest) {
  $tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP ("mahsadl-" + [guid]::NewGuid().ToString().Substring(0, 8)))
  $zip = Join-Path $tmp.FullName "dl.zip"
  Write-Host "Downloading $Url"
  Invoke-WebRequest -UserAgent "MahsaDesk" -Uri $Url -OutFile $zip
  Expand-Archive -Path $zip -DestinationPath $tmp.FullName -Force
  return $tmp.FullName
}

# #25: pin the sha256 of every shipped engine so the app's requirements card
# can show a green "verified" badge (marker format: sha256:<hex>)
function Write-Sha256Marker([string]$File) {
  if (-not (Test-Path $File)) { return }
  $h = (Get-FileHash -Algorithm SHA256 -Path $File).Hash.ToLower()
  Set-Content -Path ($File + ".sha") -Value ("sha256:" + $h) -NoNewline -Encoding ascii
  Write-Host "pinned sha256 $(Split-Path $File -Leaf)"
}

# Psiphon markers predate that scheme: they hold the git blob sha1 of the
# binary, matching the Psiphon-Labs contents API (sha1("blob <len>\0" + data))
function Write-GitBlobMarker([string]$File, [string]$MarkerPath) {
  if (-not (Test-Path $File)) { return }
  $bytes = [IO.File]::ReadAllBytes($File)
  $sha1 = [Security.Cryptography.SHA1]::Create()
  $hdr = [Text.Encoding]::ASCII.GetBytes("blob " + $bytes.Length + [char]0)
  $all = New-Object byte[] ($hdr.Length + $bytes.Length)
  [Array]::Copy($hdr, 0, $all, 0, $hdr.Length)
  [Array]::Copy($bytes, 0, $all, $hdr.Length, $bytes.Length)
  $hash = [BitConverter]::ToString($sha1.ComputeHash($all)).Replace("-", "").ToLower()
  Set-Content -Path $MarkerPath -Value $hash -NoNewline -Encoding ascii
  Write-Host "pinned git sha1 $(Split-Path $File -Leaf)"
}

# 1. Xray core - Mahsa fork (same core MahsaNG ships; tolerated configs verified v0.1)
$xrayUrl = "https://github.com/GFW-knocker/Xray-core/releases/latest/download/Xray-windows-64.zip"
$xrayDir = Expand-ZipTo $xrayUrl ""
$exe = Get-ChildItem $xrayDir -Filter "xray.exe" | Select-Object -First 1
Copy-Item $exe.FullName -Destination (Join-Path $OutDir "xray.exe") -Force
Write-Sha256Marker (Join-Path $OutDir "xray.exe")
foreach ($f in @("geoip.dat", "geosite.dat")) {
  $src = Get-ChildItem $xrayDir -Filter $f -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($src) {
    Copy-Item $src.FullName -Destination (Join-Path $OutDir $f) -Force
    Write-Sha256Marker (Join-Path $OutDir $f)
  }
}

# 2. sing-box for experimental TUN (pinned 1.10.7 classic config format, "latest" row moved past 1.10)
$singUrl = "https://github.com/SagerNet/sing-box/releases/download/v1.10.7/sing-box-1.10.7-windows-amd64.zip"
$singDir = Expand-ZipTo $singUrl ""
$sexe = Get-ChildItem $singDir -Recurse -Filter "sing-box.exe" | Select-Object -First 1
Copy-Item $sexe.FullName -Destination (Join-Path $OutDir "sing-box.exe") -Force
Write-Sha256Marker (Join-Path $OutDir "sing-box.exe")
# 2b. wintun.dll - the sing-box archive no longer ships it (sing-box embeds its
#     own copy), but the app's requirements card still lists it as a runtime
#     requirement, so fetch the file sing-tun keeps in-tree, pinned to a commit.
$wt = Get-ChildItem $singDir -Recurse -Filter "wintun.dll" -ErrorAction SilentlyContinue | Where-Object { $_.Length -gt 100000 } | Select-Object -First 1
if ($wt) {
  Copy-Item $wt.FullName -Destination (Join-Path $OutDir "wintun.dll") -Force
} else {
  $wuUrl = "https://raw.githubusercontent.com/SagerNet/sing-tun/611d83b7a57b4719be45581eb674942ff561de6c/internal/wintun/amd64/wintun.dll"
  Write-Host "Downloading $wuUrl"
  Invoke-WebRequest -UserAgent "MahsaDesk" -Uri $wuUrl -OutFile (Join-Path $OutDir "wintun.dll")
}
Write-Sha256Marker (Join-Path $OutDir "wintun.dll")

# 3. Psiphon tunnel engine - official Psiphon-Labs release-candidate binary
#    (rolling branch, i686 build runs natively on x64 Windows via WoW64)
$psiUrl = "https://raw.githubusercontent.com/Psiphon-Labs/psiphon-tunnel-core-binaries/master/windows/psiphon-tunnel-core-i686.exe"
$psiDest = Join-Path $OutDir "psiphon-tunnel-core.exe"
if (-not (Test-Path $psiDest) -or (Get-Item $psiDest).Length -lt 1MB) {
  Write-Host "Downloading $psiUrl"
  Invoke-WebRequest -UserAgent "MahsaDesk" -Uri $psiUrl -OutFile $psiDest
} else {
  Write-Host "psiphon-tunnel-core.exe already present, skipping"
}
Write-GitBlobMarker $psiDest (Join-Path $OutDir "psiphon-tunnel-core.sha")

# Fail the build instead of shipping a bundle the app will report as
# incomplete: a silently skipped file here reached every release as
# "wintun.dll - missing" plus a failing in-app upgrade.
$required = @("xray.exe", "geoip.dat", "geosite.dat", "sing-box.exe", "wintun.dll", "psiphon-tunnel-core.exe")
$missing = $required | Where-Object {
  $p = Join-Path $OutDir $_
  (-not (Test-Path $p)) -or ((Get-Item $p).Length -lt 1024)
}
if ($missing) {
  throw "missing required binaries in ${OutDir}: $($missing -join ', ')"
}

Write-Host "`nBinaries in $OutDir :"
Get-ChildItem $OutDir | Format-Table Name, Length

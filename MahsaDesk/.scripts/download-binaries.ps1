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

# 1. Xray core - Mahsa fork (same core MahsaNG ships; tolerated configs verified v0.1)
$xrayUrl = "https://github.com/GFW-knocker/Xray-core/releases/latest/download/Xray-windows-64.zip"
$xrayDir = Expand-ZipTo $xrayUrl ""
$exe = Get-ChildItem $xrayDir -Filter "xray.exe" | Select-Object -First 1
Copy-Item $exe.FullName -Destination (Join-Path $OutDir "xray.exe") -Force
foreach ($f in @("geoip.dat", "geosite.dat")) {
  $src = Get-ChildItem $xrayDir -Filter $f -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($src) { Copy-Item $src.FullName -Destination (Join-Path $OutDir $f) -Force }
}

# 2. sing-box for experimental TUN (pinned 1.10.7 classic config format, "latest" row moved past 1.10)
$singUrl = "https://github.com/SagerNet/sing-box/releases/download/v1.10.7/sing-box-1.10.7-windows-amd64.zip"
$singDir = Expand-ZipTo $singUrl ""
$sexe = Get-ChildItem $singDir -Recurse -Filter "sing-box.exe" | Select-Object -First 1
Copy-Item $sexe.FullName -Destination (Join-Path $OutDir "sing-box.exe") -Force
# wintun.dll rides along in the sing-box archive
$wt = Get-ChildItem $singDir -Recurse -Filter "wintun.dll" | Where-Object { $_.Length -gt 100000 } | Select-Object -First 1
if ($wt) { Copy-Item $wt.FullName -Destination (Join-Path $OutDir "wintun.dll") -Force }

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

Write-Host "`nBinaries in $OutDir :"
Get-ChildItem $OutDir | Format-Table Name, Length

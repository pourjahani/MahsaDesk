$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$root = "$PSScriptRoot/../.."
$srcPath = Join-Path $root "Mahsa_logo.jpg"
$outDir = "$PSScriptRoot/../src-tauri/icons"

$src = [System.Drawing.Image]::FromFile($srcPath)
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$bmps = @()
foreach ($s in $sizes) {
  $bmp = New-Object System.Drawing.Bitmap $s, $s
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.DrawImage($src, 0, 0, $s, $s)
  $bmps += , $bmp
}

$pngs = @()
foreach ($b in $bmps) {
  $pngMs = New-Object System.IO.MemoryStream
  $b.Save($pngMs, [System.Drawing.Imaging.ImageFormat]::Png)
  $pngs += , $pngMs
}

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $ms
$offset = 6 + (16 * $sizes.Count)
$bw.Write([UInt16]0)
$bw.Write([UInt16]1)
$bw.Write([UInt16]$sizes.Count)
for ($i = 0; $i -lt $sizes.Count; $i++) {
  $s = $sizes[$i]
  $bw.Write([Byte]([Math]::Min($s, 255)))  # width
  $bw.Write([Byte]([Math]::Min($s, 255)))  # height
  $bw.Write([Byte]0)  # palette
  $bw.Write([Byte]0)
  $bw.Write([UInt16]1)  # planes
  $bw.Write([UInt16]32) # bpp
  $bw.Write([UInt32]$pngs[$i].Length)
  $bw.Write([UInt32]$offset)
  $offset += $pngs[$i].Length
}
foreach ($p in $pngs) { $bw.Write($p.ToArray()) }

[System.IO.File]::WriteAllBytes((Join-Path $outDir "icon.ico"), $ms.ToArray())
$bmps[2].Save((Join-Path $outDir "32x32.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$bmps[6].Save((Join-Path $outDir "128x128.png"), [System.Drawing.Imaging.ImageFormat]::Png)
Write-Host "ICON_OK"

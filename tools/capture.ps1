param(
  [string]$IdleShot,
  [string]$HoverShot,
  [switch]$Settings
)

Add-Type -AssemblyName System.Drawing
Add-Type -Path (Join-Path $PSScriptRoot "FanalWin.cs")
[void][FanalWin]::SetProcessDPIAware()

function Find-Fanal {
  $script:hit = $null
  $cb = [FanalWin+EnumProc]{
    param($h, $l)
    $sb = New-Object System.Text.StringBuilder 512
    [void][FanalWin]::GetWindowText($h, $sb, 512)
    if ($sb.ToString() -like "Fanal*") {
      $r = New-Object FanalWin+RECT
      [void][FanalWin]::GetWindowRect($h, [ref]$r)
      $script:hit = [pscustomobject]@{
        Title = $sb.ToString(); Hwnd = $h
        Left = $r.Left; Top = $r.Top
        W = ($r.Right - $r.Left); H = ($r.Bottom - $r.Top)
      }
    }
    return $true
  }
  [FanalWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  return $script:hit
}

function Save-Win([IntPtr]$hwnd, [string]$path) {
  $r = New-Object FanalWin+RECT
  [void][FanalWin]::GetWindowRect($hwnd, [ref]$r)
  $w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
  if ($w -lt 1 -or $hgt -lt 1) { return }
  $bmp = New-Object System.Drawing.Bitmap $w, $hgt
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][FanalWin]::PrintWindow($hwnd, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
}

$cur = New-Object FanalWin+POINT
[void][FanalWin]::GetCursorPos([ref]$cur)
Write-Output "cursor was $($cur.X),$($cur.Y)"

function Wait-Settled([int]$wantW, [int]$minW, [int]$maxW) {
  $stable = $null
  $last = ''
  for ($i = 0; $i -lt 16; $i++) {
    Start-Sleep -Milliseconds 250
    $win = Find-Fanal
    if (-not $win) { continue }
    $key = "$($win.W)x$($win.H)"
    $fits = ($wantW -lt 0 -or [Math]::Abs($win.W - $wantW) -le 40) -and $win.W -ge $minW -and $win.W -le $maxW
    if ($key -eq $last -and $fits) { $stable = $win; break }
    $last = $key
  }
  if (-not $stable) { $stable = Find-Fanal }
  return $stable
}

[void][FanalWin]::SetCursorPos(20, 700)
$away = Wait-Settled -1 120 220
if (-not $away) { Write-Output "AWAY: no window"; exit 1 }
Write-Output "AWAY: $($away.Title) $($away.W)x$($away.H) @ $($away.Left),$($away.Top)"
if ($IdleShot) { Save-Win ([IntPtr]$away.Hwnd) $IdleShot }

$hx = $away.Left + [Math]::Max(8, [int]($away.W / 2))
$hy = $away.Top + 6
[void][FanalWin]::SetCursorPos($hx, $hy)
$over = Wait-Settled 498 400 700
Write-Output "OVER: $($over.Title) $($over.W)x$($over.H) @ $($over.Left),$($over.Top)"
if ($HoverShot) { Save-Win ([IntPtr]$over.Hwnd) $HoverShot }

if ($Settings -and $over) {
  $gx = $over.Left + $over.W - 36
  $gy = $over.Top + $over.H - 28
  [void][FanalWin]::SetCursorPos($gx, $gy)
  Start-Sleep -Milliseconds 150
  [FanalWin]::mouse_event([FanalWin]::MouseLeftDown, 0, 0, 0, [System.UIntPtr]::Zero)
  [FanalWin]::mouse_event([FanalWin]::MouseLeftUp, 0, 0, 0, [System.UIntPtr]::Zero)
  $page = Wait-Settled 498 400 900
  Write-Output "SETTINGS: $($page.Title) $($page.W)x$($page.H)"
  if ($page) { Save-Win ([IntPtr]$page.Hwnd) 'shot-settings.png' }
}

[void][FanalWin]::SetCursorPos($cur.X, $cur.Y)
Write-Output "cursor restored"

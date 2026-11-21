param(
  [string]$IdleShot,
  [string]$HoverShot
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

[void][FanalWin]::SetCursorPos(20, 700)
Start-Sleep -Milliseconds 1000
$away = Find-Fanal
if (-not $away) { Write-Output "AWAY: no window"; exit 1 }
Write-Output "AWAY: $($away.Title) $($away.W)x$($away.H) @ $($away.Left),$($away.Top)"
if ($IdleShot) { Save-Win ([IntPtr]$away.Hwnd) $IdleShot }

$hx = $away.Left + [Math]::Max(10, [int]($away.W / 2))
$hy = $away.Top + [Math]::Min(8, [Math]::Max(2, [int]($away.H / 2)))
[void][FanalWin]::SetCursorPos($hx, $hy)
Start-Sleep -Milliseconds 800
$over = Find-Fanal
Write-Output "OVER: $($over.Title) $($over.W)x$($over.H) @ $($over.Left),$($over.Top)"
if ($HoverShot) { Save-Win ([IntPtr]$over.Hwnd) $HoverShot }

[void][FanalWin]::SetCursorPos($cur.X, $cur.Y)
Write-Output "cursor restored"

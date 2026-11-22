# Turns the downloaded brand marks into the 32px tiles the panel shows.
# Sources are the MIT-licensed marks from lobehub/lobe-icons, plus Orca's own
# logo, which that set does not carry.
$src = Join-Path $PSScriptRoot "..\ui\marks\src"
$out = Join-Path $PSScriptRoot "..\ui\marks"

function Read-Mark([string]$name) {
    $text = [System.IO.File]::ReadAllText((Join-Path $src "$name.svg"))
    $text = $text -replace '(?s)<title>.*?</title>', ''
    $text = $text -replace '\sstyle="[^"]*"', ''
    $text = $text -replace '\s(?:width|height)="[^"]*"', ''
    $text = $text -replace '\sxml:space="[^"]*"', ''
    return $text.Trim()
}

function Add-AttrIfMissing([string]$markup, [string]$name, [string]$value) {
    if (-not $value) { return $markup }
    $rx = [regex]'<(path|circle|rect|polygon|polyline|ellipse)\b([^>]*)>'
    $sb = New-Object System.Text.StringBuilder
    $last = 0
    foreach ($m in $rx.Matches($markup)) {
        [void]$sb.Append($markup.Substring($last, $m.Index - $last))
        $tag = $m.Groups[1].Value
        $rest = $m.Groups[2].Value
        if ($rest -match ('\b' + [regex]::Escape($name) + '\s*=')) {
            [void]$sb.Append($m.Value)
        } else {
            $gap = ''
            if ($rest.Length -gt 0 -and -not $rest.StartsWith(' ')) { $gap = ' ' }
            [void]$sb.Append("<$tag $name=`"$value`"$gap$rest>")
        }
        $last = $m.Index + $m.Length
    }
    if ($last -lt $markup.Length) { [void]$sb.Append($markup.Substring($last)) }
    return $sb.ToString()
}

function Convert-ToTile([string]$svg, [string]$background, [string]$ink, [int]$pad) {
    $viewBox = '0 0 24 24'
    $rootFill = ''
    $rootRule = ''
    if ($svg -match '<svg\b([^>]*)>') {
        $attrs = $Matches[1]
        if ($attrs -match 'viewBox="([^"]+)"') { $viewBox = $Matches[1] }
        if ($attrs -match '\bfill="([^"]+)"') { $rootFill = $Matches[1] }
        if ($attrs -match '\bfill-rule="([^"]+)"') { $rootRule = $Matches[1] }
    }
    $inner = $svg -replace '(?s)^.*?<svg[^>]*>', '' -replace '</svg>\s*$', ''
    # A nested svg does not hand its own fill down to the shapes, and several
    # of these marks only name a colour on the root. Stamp it onto each shape
    # that does not already have one, and leave a real colour (Kimi's blue,
    # Trae's green) alone.
    $paint = $ink
    if (-not $paint -and $rootFill -and $rootFill -ne 'currentColor') { $paint = $rootFill }
    if ($paint) { $inner = $inner.Replace('currentColor', $paint) }
    $inner = Add-AttrIfMissing $inner 'fill' $paint
    $inner = Add-AttrIfMissing $inner 'fill-rule' $rootRule
    $origin = $pad
    $size = 32 - (2 * $pad)
    $back = ''
    if ($background) { $back = "<rect width=`"32`" height=`"32`" rx=`"8`" fill=`"$background`"/>" }
    return "<svg xmlns=`"http://www.w3.org/2000/svg`" viewBox=`"0 0 32 32`">$back<svg x=`"$origin`" y=`"$origin`" width=`"$size`" height=`"$size`" viewBox=`"$viewBox`">$inner</svg></svg>"
}

# background, ink (empty keeps the mark's own colours), padding
$tiles = @{
    'claude'       = @('#D97757', '#fff', 5)
    'cursor'       = @('#111111', '#fff', 5)
    'deepseek'     = @('#4D6BFE', '#fff', 4)
    'codex'        = @('', '', 0)
    'kimi'         = @('#ffffff', '#111111', 4)
    'chatgpt'      = @('#111111', '#fff', 4)
    'gemini'       = @('#ffffff', '', 4)
    'trae'         = @('#111111', '', 5)
    'kiro'         = @('#ffffff', '#111111', 4)
    'qoder'        = @('#ffffff', '#111111', 3)
    'antigravity'  = @('#ffffff', '#111111', 5)
    'opencode'     = @('#111111', '#fff', 5)
    'devin'        = @('#ffffff', '', 3)
}

foreach ($name in $tiles.Keys) {
    $spec = $tiles[$name]
    $svg = Read-Mark $name
    if ($spec[1]) {
        $svg = $svg.Replace('#D97757', $spec[1]).Replace('#4D6BFE', $spec[1])
    }
    $tile = Convert-ToTile $svg $spec[0] $spec[1] ([int]$spec[2])
    [System.IO.File]::WriteAllText((Join-Path $out "$name.svg"), $tile, (New-Object System.Text.UTF8Encoding $false))
}

# DeepSeek Harness ships the DeepSeek mark; Kimi Code ships the Kimi mark.
Copy-Item (Join-Path $out 'deepseek.svg') (Join-Path $out 'deepseek-harness.svg') -Force
Copy-Item (Join-Path $out 'kimi.svg') (Join-Path $out 'kimi-code.svg') -Force

# Orca's own wordless whale, from the installed app. It is drawn in white.
$orcaPath = "$env:LOCALAPPDATA\Programs\orca\resources\app.asar.unpacked\resources\logo.svg"
$orca = [System.IO.File]::ReadAllText($orcaPath)
$orca = $orca -replace '(?s)<\?xml.*?\?>', ''
$orca = $orca -replace '(?s)<!--.*?-->', ''
# The whale is painted white through a style attribute, which the inliner strips.
$orca = $orca -replace 'style="[^"]*fill:\s*(#[0-9A-Fa-f]+)[^"]*"', 'fill="$1"'
$tile = Convert-ToTile $orca.Trim() '#0E2433' '' 4
[System.IO.File]::WriteAllText((Join-Path $out 'orca.svg'), $tile, (New-Object System.Text.UTF8Encoding $false))

& (Join-Path $PSScriptRoot 'inline-marks.ps1')

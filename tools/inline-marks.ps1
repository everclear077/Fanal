# Reads ui/marks/*.svg and writes a JS object literal the panel inlines.
$dir = Join-Path $PSScriptRoot "..\ui\marks"
$out = New-Object System.Text.StringBuilder
[void]$out.AppendLine("const MARKS = {")
Get-ChildItem $dir -Filter *.svg | Sort-Object Name | ForEach-Object {
    $id = $_.BaseName
    $svg = [System.IO.File]::ReadAllText($_.FullName).Trim()
    $svg = $svg -replace 'xmlns="http://www.w3.org/2000/svg"', ''
    $svg = $svg -replace '<svg\s+', '<svg xmlns="http://www.w3.org/2000/svg" '
    $svg = $svg -replace '\s+', ' '
    $escaped = $svg.Replace('\', '\\').Replace('"', '\"')
    [void]$out.AppendLine(('  "{0}": "{1}",' -f $id, $escaped))
}
[void]$out.AppendLine("};")
[System.IO.File]::WriteAllText((Join-Path $PSScriptRoot "..\ui\marks.js"), $out.ToString(), (New-Object System.Text.UTF8Encoding($false)))
Write-Output "wrote marks.js"

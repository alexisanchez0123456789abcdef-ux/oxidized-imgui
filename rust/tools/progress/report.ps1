# Generates docs/ESTADO.md from docs/progress.json and prints the current percentage.
#
# The percentage is a rubric, not a measurement: it only counts what has evidence. Editing a
# "done" flag without evidence is a lie the panel will show, so the JSON asks for the evidence
# next to it.
#
#   pwsh report.ps1          # rewrite docs/ESTADO.md
#   pwsh report.ps1 -Check   # fail if docs/ESTADO.md is out of date (used by CI)

param([switch]$Check)

$ErrorActionPreference = 'Stop'
$invariant = [System.Globalization.CultureInfo]::InvariantCulture

$root   = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$source = Join-Path $root 'docs\progress.json'
$target = Join-Path $root 'docs\ESTADO.md'

$data = Get-Content $source -Raw -Encoding UTF8 | ConvertFrom-Json

$totalWeight = 0
$doneWeight = 0
$rewriteWeight = 0
$rewriteDone = 0
$rows = @()

foreach ($area in $data.areas) {
    $areaWeight = [int]$area.weight
    $itemsWeight = 0
    $itemsDone = 0
    foreach ($item in $area.items) {
        $itemsWeight += [int]$item.weight
        if ($item.done) { $itemsDone += [int]$item.weight }
    }
    if ($itemsWeight -ne $areaWeight) {
        throw "area '$($area.id)': item weights sum $itemsWeight but the area declares $areaWeight"
    }
    $totalWeight += $areaWeight
    $doneWeight += $itemsDone
    if (-not $area.infra) {
        $rewriteWeight += $areaWeight
        $rewriteDone += $itemsDone
    }
    $percent = if ($areaWeight -gt 0) { $itemsDone * 100.0 / $areaWeight } else { 0 }
    $rows += [pscustomobject]@{
        title   = $area.title
        weight  = $areaWeight
        percent = [math]::Round($percent, 0)
        infra   = [bool]$area.infra
    }
}

$rewrite = [math]::Round($rewriteDone * 100.0 / $rewriteWeight, 1).ToString('0.0', $invariant)
$project = [math]::Round($doneWeight * 100.0 / $totalWeight, 1).ToString('0.0', $invariant)

$verified = @()
$pending = @()
foreach ($area in $data.areas) {
    foreach ($item in $area.items) {
        if ($item.done) { $verified += $item }
        elseif (-not $area.infra) { $pending += $item }
    }
}
$pending = $pending | Sort-Object { -[int]$_.weight } | Select-Object -First 3

$lines = @()
$lines += '# Estado del proyecto'
$lines += ''
$lines += "_Actualizado: $($data.updated). Generado desde ``docs/progress.json``: no editar a mano._"
$lines += ''
$lines += "## Reescritura a Rust: **$rewrite %**"
$lines += ''
$lines += "Del trabajo de reimplementación, esto es lo que está hecho **y demostrado**. El total del proyecto, contando la infraestructura que lo verifica, va al $project %."
$lines += ''
$lines += 'Qué cuenta como hecho: solo lo que tiene una prueba que lo compara con el código C++ original.'
$lines += 'Un cambio de estado sin evidencia no sube el porcentaje.'
$lines += ''
$lines += '| Área | Peso | Avance |'
$lines += '| --- | ---: | ---: |'
foreach ($row in $rows) {
    $lines += "| $($row.title) | $($row.weight) | $($row.percent) % |"
}
$lines += ''
$lines += '## Verificado'
$lines += ''
foreach ($item in $verified) {
    $lines += "- **$($item.label)** — $($item.evidence)"
}
$lines += ''
$lines += '## Lo siguiente, por peso'
$lines += ''
foreach ($item in $pending) {
    $lines += "- $($item.label) (peso $($item.weight))"
}
$lines += ''
$lines += '## Cómo se actualiza'
$lines += ''
$lines += '`docs/progress.json` es la fuente de la verdad. Cambiar un `done` obliga a escribir su evidencia;'
$lines += 'el CI regenera esta página y falla si no coincide, así que el porcentaje no puede separarse de la realidad.'
$lines += ''

$markdown = $lines -join "`n"

if ($Check) {
    if (-not (Test-Path $target)) {
        Write-Host 'FAIL: docs/ESTADO.md does not exist; run report.ps1'
        exit 1
    }
    $current = [IO.File]::ReadAllText($target)
    if ($current -ne $markdown) {
        Write-Host 'FAIL: docs/ESTADO.md is out of date; run report.ps1 and commit the result'
        exit 1
    }
    Write-Host 'PASS: docs/ESTADO.md matches docs/progress.json'
    exit 0
}

[IO.File]::WriteAllText($target, $markdown, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "rewrite $rewrite %  project $project %"
Write-Host "written: $target"

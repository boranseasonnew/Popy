param([switch]$WithGpu)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
    $popy = Join-Path $projectRoot 'target\release\popy.exe'
    if (!(Test-Path -LiteralPath $popy)) { throw 'Build first: .\scripts\dev.ps1 build --release' }
    $runtime = Get-Content -LiteralPath '.tools\runtime.json' -Raw | ConvertFrom-Json
    $testRoot = Join-Path $projectRoot ('target\smoke\' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    $fixtureDirectory = Join-Path $testRoot 'fixtures'
    & $popy fixtures --out $fixtureDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Fixture generation failed' }
    $source = Join-Path $fixtureDirectory 'input-144p.mp4'
    $sourceHash = (Get-FileHash -LiteralPath $source).Hash
    & $popy inspect --input $source | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Valid CFR source was rejected' }
    $vfr = Join-Path $testRoot 'invalid-vfr.mp4'
    & $runtime.ffmpeg -v error -nostdin -n -i $source -vf "select='not(eq(n,5))'" -fps_mode vfr -c:v libx264 -c:a copy -color_primaries bt709 -color_trc bt709 -colorspace bt709 -color_range tv $vfr
    if ($LASTEXITCODE -ne 0) { throw 'VFR fixture failed' }
    & $popy inspect --input $vfr
    if ($LASTEXITCODE -eq 0) { throw 'VFR was silently accepted' }
    $hdr = Join-Path $testRoot 'invalid-hdr.mp4'
    & $runtime.ffmpeg -v error -nostdin -n -i $source -c copy -bsf:v 'h264_metadata=transfer_characteristics=16' -color_trc smpte2084 $hdr
    if ($LASTEXITCODE -ne 0) { throw 'HDR fixture failed' }
    & $popy inspect --input $hdr
    if ($LASTEXITCODE -eq 0) { throw 'HDR was silently accepted' }
    if ($WithGpu) {
        $destination = Join-Path $testRoot 'Türkçe çıktı; boşluk'
        & $popy benchmark --input $source --out $destination --model anime --seconds 1 --gpu 0 --tile 64 --rights 'Generated technical fixture' --auto-color
        if ($LASTEXITCODE -ne 0) { throw 'GPU/color/Unicode-path smoke test failed' }
        $report = Get-Content -LiteralPath (Join-Path $destination 'report.json') -Raw | ConvertFrom-Json
        if ($report.state -ne 'completed' -or $report.validation.aac_payload_matches_source -ne $true) { throw 'Output verification failed' }
        & $popy benchmark --input $source --out $destination --model anime --seconds 1 --rights 'Generated fixture'
        if ($LASTEXITCODE -eq 0) { throw 'Existing output was overwritten' }
    }
    if ((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash) { throw 'Source changed' }
    Write-Output "Smoke checks passed. Artifacts: $testRoot"
} finally { Pop-Location }

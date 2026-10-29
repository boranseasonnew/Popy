param([switch]$WithDesktop)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
    $popy = Join-Path $projectRoot 'target\release\popy.exe'
    $desktop = Join-Path $projectRoot 'target\release\popy-desktop.exe'
    $runtime = Get-Content -LiteralPath '.tools\runtime.json' -Raw | ConvertFrom-Json
    $testRoot = Join-Path $projectRoot ('target\export-smoke\' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    function New-Clip([string]$Path, [string]$Rate, [string]$Seconds, [switch]$Silent) {
        $clipArgs = @('-v','error','-nostdin','-n','-f','lavfi','-i',"testsrc2=size=128x72:rate=${Rate}:duration=$Seconds")
        if (!$Silent) { $clipArgs += @('-f','lavfi','-i',"sine=frequency=880:sample_rate=48000:duration=$Seconds",'-c:a','aac') }
        $clipArgs += @('-vf','setparams=range=limited:color_primaries=bt709:color_trc=bt709:colorspace=bt709','-c:v','libx264','-crf','24','-pix_fmt','yuv420p','-color_primaries','bt709','-color_trc','bt709','-colorspace','bt709','-color_range','tv',$Path)
        & $runtime.ffmpeg @clipArgs
        if ($LASTEXITCODE -ne 0) { throw 'Fixture encode failed' }
    }
    $long = Join-Path $testRoot 'source-33s.mp4'
    New-Clip $long '12' '33'
    $hash = (Get-FileHash -LiteralPath $long).Hash
    $output = Join-Path $testRoot 'Uzun çıktı Türkçe; test'
    & $popy enhance --input $long --out $output --model anime --auto-color
    if ($LASTEXITCODE -ne 0) { throw '33-second multi-batch export failed' }
    $report = Get-Content -LiteralPath (Join-Path $output 'report.json') -Raw | ConvertFrom-Json
    if ($report.state -ne 'completed' -or $report.output_media.frame_count -ne 396 -or $report.validation.aac_payload_matches -ne $true -or $report.engine.batches -le 1) { throw 'Long export verification failed' }
    if (Test-Path -LiteralPath (Join-Path $output 'work')) { throw 'Successful export left temporary frames/segments' }
    if ((Get-FileHash -LiteralPath $long).Hash -ne $hash) { throw 'Source changed' }
    & $popy enhance --input $long --out $output --model anime
    if ($LASTEXITCODE -eq 0) { throw 'Existing output was overwritten' }

    $fractional = Join-Path $testRoot 'fractional-silent.mp4'
    New-Clip $fractional '30000/1001' '3' -Silent
    $fractionOutput = Join-Path $testRoot 'fractional'
    & $popy enhance --input $fractional --out $fractionOutput --model anime
    if ($LASTEXITCODE -ne 0) { throw 'Fractional/silent export failed' }
    $fractionReport = Get-Content -LiteralPath (Join-Path $fractionOutput 'report.json') -Raw | ConvertFrom-Json
    if ($fractionReport.output_media.rate.numerator -ne 30000 -or $fractionReport.output_media.rate.denominator -ne 1001 -or $fractionReport.output_media.has_audio) { throw 'Fractional FPS or silence changed' }

    # Exercise cancellation while ncnn is actually running, rather than only
    # setting a marker before the worker starts.
    $cancel = Join-Path $testRoot 'cancel.flag'
    $cancelOutput = Join-Path $testRoot 'cancelled'
    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = $popy
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.WorkingDirectory = $projectRoot
    foreach ($arg in @('enhance','--input',$long,'--out',$cancelOutput,'--model','anime','--cancel-file',$cancel)) { $startInfo.ArgumentList.Add($arg) }
    $process = [Diagnostics.Process]::Start($startInfo)
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 100
        $statePath = Join-Path $cancelOutput 'job.json'
        $state = if (Test-Path -LiteralPath $statePath) { Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json } else { $null }
    } until ($state.stage -eq 'upscale' -or $process.HasExited -or [DateTime]::UtcNow -gt $deadline)
    if ($state.stage -ne 'upscale') { throw 'Cancellation test never reached inference' }
    Start-Sleep -Milliseconds 300
    [IO.File]::WriteAllText($cancel,'cancel')
    if (!$process.WaitForExit(10000)) { $process.Kill($true); throw 'Cancelled worker did not exit' }
    $cancelState = Get-Content -LiteralPath (Join-Path $cancelOutput 'job.json') -Raw | ConvertFrom-Json
    if ($process.ExitCode -eq 0 -or $cancelState.state -ne 'cancelled' -or (Test-Path -LiteralPath (Join-Path $cancelOutput 'enhanced.mp4'))) { throw 'Cancellation falsely reported success' }
    $process.Dispose()

    if ($WithDesktop) {
        $desktopInfo = New-Object Diagnostics.ProcessStartInfo
        $desktopInfo.FileName = $desktop
        $desktopInfo.UseShellExecute = $false
        $desktopInfo.CreateNoWindow = $true
        $desktopInfo.WorkingDirectory = $projectRoot
        foreach ($arg in @('--smoke-input',$fractional,'--smoke-output',(Join-Path $testRoot 'desktop-exports'),'--smoke-screenshot',(Join-Path $testRoot 'desktop.png'))) { $desktopInfo.ArgumentList.Add($arg) }
        $window = [Diagnostics.Process]::Start($desktopInfo)
        if (!$window.WaitForExit(120000)) { $window.Kill($true); throw 'Desktop smoke timed out' }
        if ($window.ExitCode -ne 0 -or !(Test-Path -LiteralPath (Join-Path $testRoot 'desktop.png'))) { throw 'Desktop worker/screenshot smoke failed' }
        $window.Dispose()
    }
    Write-Output "Export smoke passed. Artifacts: $testRoot"
} finally { Pop-Location }

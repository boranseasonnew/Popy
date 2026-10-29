$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$checkRoot = Join-Path $projectRoot ('target\updater-smoke\' + [guid]::NewGuid().ToString('N'))
$ps5 = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$helper = Join-Path $PSScriptRoot 'update-install.ps1'
$utf8 = New-Object Text.UTF8Encoding($false)
function Test-Install([string]$Name,[string[]]$Files,[string]$ExpectedState) {
    $app = Join-Path $checkRoot "$Name\Popy Türkçe"
    $staged = Join-Path $checkRoot "$Name\staged"
    New-Item -ItemType Directory -Path (Join-Path $app 'exports'),$staged -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $app 'popy.exe'),'old-cli',$utf8)
    [IO.File]::WriteAllText((Join-Path $app 'popy-desktop.exe'),'old-gui',$utf8)
    [IO.File]::WriteAllText((Join-Path $app 'exports\private.mp4'),'private-data',$utf8)
    [IO.File]::WriteAllText((Join-Path $staged 'popy.exe'),'new-cli',$utf8)
    [IO.File]::WriteAllText((Join-Path $staged 'popy-desktop.exe'),'new-gui',$utf8)
    $config = Join-Path $checkRoot "$Name\install.json"
    $json = @{root=$app;staged=$staged;files=$Files;version='9.9.9';parent_pid=-1} | ConvertTo-Json -Depth 5
    [IO.File]::WriteAllText($config,$json,$utf8)
    & $ps5 -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $helper -Config $config -NoRestart
    if ($LASTEXITCODE -ne 0) { throw 'Update helper failed unexpectedly' }
    $result = Get-Content -LiteralPath (Join-Path $app '.tools\update-result.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($result.state -ne $ExpectedState) { throw "Unexpected updater state: $($result.state)" }
    if ((Get-Content -LiteralPath (Join-Path $app 'exports\private.mp4') -Raw) -ne 'private-data') { throw 'Updater changed user data' }
    $expected = if ($ExpectedState -eq 'completed') {'new-cli'} else {'old-cli'}
    if ((Get-Content -LiteralPath (Join-Path $app 'popy.exe') -Raw) -ne $expected) { throw 'Install/rollback contents incorrect' }
    if ($ExpectedState -eq 'failed' -and $result.rollback_errors.Count -gt 0) { throw 'Rollback reported errors' }
}
Test-Install 'success' @('popy.exe','popy-desktop.exe') 'completed'
Test-Install 'rollback' @('popy.exe','missing.exe') 'failed'
Test-Install 'protected-data' @('popy.exe','exports/private.mp4') 'failed'
Test-Install 'path-escape' @('popy.exe','../escape.exe') 'failed'
Write-Output "Updater install/rollback/protected-data/path tests passed: $checkRoot"

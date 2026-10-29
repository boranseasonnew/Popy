param([string]$Compiler = '', [string]$Version = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = (& (Join-Path $projectRoot 'target\release\popy.exe') --version).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read release version' }
}
if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?$') { throw 'Invalid version' }
if ([string]::IsNullOrWhiteSpace($Compiler)) { $Compiler = Join-Path $projectRoot '.tools\innosetup\ISCC.exe' }
if (!(Test-Path -LiteralPath $Compiler)) { throw 'Pass -Compiler with the path to an official Inno Setup 6.7+ ISCC.exe.' }
$packageRoot = Join-Path $projectRoot "dist\Popy-$Version-windows-x64"
$manifest = Get-Content -LiteralPath (Join-Path $packageRoot 'manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.version -ne $Version) { throw 'Package version mismatch' }
$outputRoot = Join-Path $projectRoot 'dist'
$destination = Join-Path $outputRoot "Popy-$Version-setup.exe"
if (Test-Path -LiteralPath $destination) { throw 'Installer already exists; inspect it before rebuilding.' }
& $Compiler '/Qp' "/DPopyVersion=$Version" "/DPackageRoot=$packageRoot" "/DOutputRoot=$outputRoot" (Join-Path $PSScriptRoot 'installer.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
$hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$destination.sha256", "$hash  $(Split-Path $destination -Leaf)`n", (New-Object Text.UTF8Encoding($false)))
Write-Output "Installer: $destination"
Write-Output "SHA-256: $hash"

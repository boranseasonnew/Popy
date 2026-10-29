param([switch]$SkipRust)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$projectRoot = Split-Path $PSScriptRoot -Parent
$toolsRoot = Join-Path $projectRoot '.tools'
$downloads = Join-Path $toolsRoot 'downloads'
New-Item -ItemType Directory -Path $downloads -Force | Out-Null

function Get-VerifiedArchive([string]$Uri, [string]$Name, [string]$ExpectedHash) {
    $archive = Join-Path $downloads $Name
    if (!(Test-Path -LiteralPath $archive)) { Invoke-WebRequest -Uri $Uri -OutFile $archive -TimeoutSec 600 }
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    if ($actual -ne $ExpectedHash) { throw "Checksum mismatch for $Name. Download is not used." }
    return $archive
}

# Hashes recorded from the upstream release downloads on 2026-10-29.
# They pin these bytes; they are not a substitute for publisher authentication.
$ffmpegUrl = 'https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip'
$ffmpegHash = '60F467265B1E312373DBCD92200C2618A74850F98D3D078E94296BB3FA2047BA'
$ffmpegArchive = Get-VerifiedArchive $ffmpegUrl 'ffmpeg-github.zip' $ffmpegHash
$ffmpegDirectory = Join-Path $toolsRoot 'ffmpeg-github'
$ffmpegBin = Join-Path $ffmpegDirectory 'ffmpeg-9.0.2-essentials_build\bin'
if (!(Test-Path -LiteralPath (Join-Path $ffmpegBin 'ffprobe.exe'))) {
    Expand-Archive -LiteralPath $ffmpegArchive -DestinationPath $ffmpegDirectory -Force
}
$modelUrl = 'https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-windows.zip'
$modelHash = 'ABC02804E17982A3BE33675E4D471E91EA374E65B70167ABC09E31ACB412802D'
$modelArchive = Get-VerifiedArchive $modelUrl 'realesrgan-models-20220424.zip' $modelHash
$modelDirectory = Join-Path $toolsRoot 'realesrgan-models'
if (!(Test-Path -LiteralPath (Join-Path $modelDirectory 'models\realesrgan-x4plus.bin'))) {
    Expand-Archive -LiteralPath $modelArchive -DestinationPath $modelDirectory -Force
}

if (!$SkipRust) {
    $env:CARGO_HOME = Join-Path $toolsRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $toolsRoot 'rustup'
    $rustup = Join-Path $env:CARGO_HOME 'bin\rustup.exe'
    if (!(Test-Path -LiteralPath $rustup)) {
        $rustupUrl = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
        $hashFile = Join-Path $downloads 'rustup-init.exe.sha256'
        Invoke-WebRequest -Uri ($rustupUrl + '.sha256') -OutFile $hashFile -TimeoutSec 120
        $rustupHash = ((Get-Content -LiteralPath $hashFile -Raw).Trim() -split '\s+')[0]
        $installer = Get-VerifiedArchive $rustupUrl 'rustup-init.exe' $rustupHash
        $env:RUSTUP_INIT_SKIP_MSVC_CHECK = 'yes'
        & $installer -y --no-modify-path --profile minimal --default-toolchain stable
        if ($LASTEXITCODE -ne 0) { throw 'Rust installation failed' }
    }
    & $rustup component add rustfmt clippy
    if ($LASTEXITCODE -ne 0) { throw 'Rust component installation failed' }
}

$runtimeJson = [ordered]@{
    # Relative to .tools/runtime.json so an installed package can be moved.
    ffmpeg = 'ffmpeg-github\ffmpeg-9.0.2-essentials_build\bin\ffmpeg.exe'
    ffprobe = 'ffmpeg-github\ffmpeg-9.0.2-essentials_build\bin\ffprobe.exe'
    ncnn = 'realesrgan-models\realesrgan-ncnn-vulkan.exe'
    models = 'realesrgan-models\models'
} | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $toolsRoot 'runtime.json'), $runtimeJson, (New-Object Text.UTF8Encoding($false)))
$provenanceJson = [ordered]@{
    downloaded_at = '2026-10-29'
    archives = @(
        @{ url=$ffmpegUrl; sha256=$ffmpegHash; license='GPL build; local research tool, not bundled in Popy executable' },
        @{ url=$modelUrl; sha256=$modelHash; license='See docs/THIRD_PARTY.md; author release with model files' }
    )
} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $toolsRoot 'provenance.json'), $provenanceJson, (New-Object Text.UTF8Encoding($false)))
if ($SkipRust) { Write-Output 'Tools ready. You can now open popy-desktop.exe.' }
else { Write-Output 'Local tools ready. Run: .\scripts\dev.ps1 test' }

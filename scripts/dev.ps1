# PowerShell may consume Cargo's literal -- separator when invoking a .ps1.
$CargoArguments = $args
if ($CargoArguments.Count -gt 1 -and $CargoArguments[0] -eq 'run' -and '--' -notin $CargoArguments) {
    $splitIndex = 1
    while ($splitIndex -lt $CargoArguments.Count -and $CargoArguments[$splitIndex] -in @('--release','--locked','--offline')) { $splitIndex++ }
    $CargoArguments = @($CargoArguments[0..($splitIndex-1)]) + @('--') + @($CargoArguments[$splitIndex..($CargoArguments.Count-1)])
}
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
$env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
$env:CARGO_INCREMENTAL = '0'
$env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
$cargoPath = Join-Path $env:CARGO_HOME 'bin\cargo.exe'
if (!(Test-Path -LiteralPath $cargoPath)) { throw 'Run scripts/bootstrap.ps1 to install local Rust first.' }
# This machine has the CRT/SDK libraries but an incomplete VS linker install.
# Prefer the regular linker when available, otherwise use Rust's bundled LLD.
if (!(Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vsWhere) {
        $instances = & $vsWhere -all -products '*' -format json | ConvertFrom-Json
        $vcRoot = $instances | ForEach-Object {
            Get-ChildItem -LiteralPath (Join-Path $_.installationPath 'VC\Tools\MSVC') -ErrorAction SilentlyContinue
        } | Sort-Object Name -Descending | Select-Object -First 1
        $sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
        $sdk = Get-ChildItem -LiteralPath $sdkRoot -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
        if ($vcRoot -and $sdk) {
            $env:LIB = (Join-Path $vcRoot.FullName 'lib\x64') + ';' + (Join-Path $sdk.FullName 'um\x64') + ';' + (Join-Path $sdk.FullName 'ucrt\x64')
            $toolchainSysroot = & (Join-Path $env:CARGO_HOME 'bin\rustc.exe') --print sysroot
            $lld = Join-Path $toolchainSysroot 'lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
            $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $lld
            $env:RUSTFLAGS = '-C target-feature=+crt-static -C linker-flavor=lld-link'
        }
    }
}
Push-Location $projectRoot
try {
    if ($CargoArguments[0] -eq 'clippy') { $env:RUSTFLAGS = ($env:RUSTFLAGS + ' -Dwarnings').Trim() }
    & $cargoPath @CargoArguments
    $cargoExit = $LASTEXITCODE
} finally { Pop-Location }
exit $cargoExit

param([string]$Version = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$binaryVersion = & (Join-Path $projectRoot 'target\release\popy.exe') --version
if ([string]::IsNullOrWhiteSpace($Version)) { $Version = $binaryVersion.Trim() }
if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?$') { throw 'Invalid version' }
if ($LASTEXITCODE -ne 0 -or $binaryVersion.Trim() -ne $Version) { throw 'Build the matching release version before packaging.' }
$packageRoot = Join-Path $projectRoot "dist\Popy-$Version-windows-x64"
if (Test-Path -LiteralPath $packageRoot) { throw 'Package directory already exists; choose a new version or inspect it first.' }
New-Item -ItemType Directory -Path (Join-Path $packageRoot 'scripts') | Out-Null
foreach ($name in @('popy.exe','popy-desktop.exe')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "target\release\$name") -Destination $packageRoot
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination $packageRoot
Copy-Item -LiteralPath (Join-Path $projectRoot 'scripts\bootstrap.ps1') -Destination (Join-Path $packageRoot 'scripts')
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\THIRD_PARTY.md') -Destination $packageRoot
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\QUICKSTART.md') -Destination (Join-Path $packageRoot 'Read me.md')
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\QUICKSTART.tr.md') -Destination (Join-Path $packageRoot 'Beni Oku.md')
$setup = "@echo off`r`npowershell.exe -NoProfile -ExecutionPolicy Bypass -File `"%~dp0scripts\bootstrap.ps1`" -SkipRust`r`nif errorlevel 1 echo Setup failed. Please review the error above.`r`npause`r`n"
[IO.File]::WriteAllText((Join-Path $packageRoot 'Araclari Kur.cmd'), $setup, (New-Object Text.UTF8Encoding($false)))
[IO.File]::WriteAllText((Join-Path $packageRoot 'Setup Tools.cmd'), $setup, (New-Object Text.UTF8Encoding($false)))

# Include the Windows dependency graph, including build/font notices. Runtime
# executables/models are NOT included in this archive.
$metadataText = & (Join-Path $PSScriptRoot 'dev.ps1') metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$metadata = ($metadataText -join "`n") | ConvertFrom-Json
$licenseRoot = Join-Path $packageRoot 'licenses'
New-Item -ItemType Directory -Path $licenseRoot | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'third_party\licenses\SOURCES.json') -Destination $licenseRoot
New-Item -ItemType Directory -Path (Join-Path $licenseRoot 'inno-setup') | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'third_party\licenses\inno-setup\LICENSE.txt') -Destination (Join-Path $licenseRoot 'inno-setup')
$nodes = @{}
foreach ($node in $metadata.resolve.nodes) { $nodes[$node.id] = $node }
$included = New-Object 'Collections.Generic.HashSet[string]'
$pending = New-Object 'Collections.Generic.Queue[string]'
$pending.Enqueue($metadata.resolve.root)
while ($pending.Count -gt 0) {
    $id = $pending.Dequeue()
    if ($included.Add($id)) { foreach ($dependencyId in $nodes[$id].dependencies) { $pending.Enqueue($dependencyId) } }
}
$inventory = @('# Locked Rust dependency notices', '', 'This inventory covers the resolved Windows graph, including build dependencies. Runtime tools and models are downloaded separately.', '')
foreach ($dependency in $metadata.packages) {
    if ($dependency.name -eq 'popy' -or !$included.Contains($dependency.id)) { continue }
    $id = "$($dependency.name)-$($dependency.version)"
    $sourceRoot = Split-Path $dependency.manifest_path -Parent
    $notices = @(Get-ChildItem -LiteralPath $sourceRoot -File -Recurse | Where-Object {
        $_.Name.ToUpperInvariant() -cmatch '^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE|OFL|UFL)([._-].*)?$' -or
        $_.Name -in @('Hack-Regular.txt','egui-icons.txt','emoji-icon-font-mit-license.txt')
    })
    $supplement = Join-Path $projectRoot "third_party\licenses\$($dependency.name)"
    if (!(Test-Path -LiteralPath $supplement) -and $dependency.repository -like 'https://github.com/emilk/egui*') {
        $supplement = Join-Path $projectRoot 'third_party\licenses\egui'
    }
    if ($notices.Count -eq 0 -and (Test-Path -LiteralPath $supplement)) {
        $sourceRoot = $supplement
        $notices = @(Get-ChildItem -LiteralPath $supplement -File)
    }
    if ($notices.Count -eq 0) { throw "No license text found for $id ($($dependency.license)). Inspect its source before packaging." }
    $destination = Join-Path $licenseRoot $id
    New-Item -ItemType Directory -Path $destination | Out-Null
    foreach ($notice in $notices) {
        $relative = $notice.FullName.Substring($sourceRoot.Length).TrimStart('\','/')
        $target = Join-Path $destination $relative
        New-Item -ItemType Directory -Path (Split-Path $target -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $notice.FullName -Destination $target
    }
    $inventory += "- $id | $($dependency.license) | $($dependency.repository)"
}
[IO.File]::WriteAllText((Join-Path $packageRoot 'DEPENDENCIES.md'), ($inventory -join "`n"), (New-Object Text.UTF8Encoding($false)))
$files = Get-ChildItem -LiteralPath $packageRoot -File -Recurse | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path=$_.FullName.Substring($packageRoot.Length+1).Replace('\','/'); bytes=$_.Length; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$revision = (& git -C $projectRoot rev-parse HEAD).Trim()
$manifest = [ordered]@{version=$Version;source_revision=$revision;platform='windows-x64';signed=$false;runtime_included=$false;files=@($files)} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $packageRoot 'manifest.json'), $manifest, (New-Object Text.UTF8Encoding($false)))
$archive = "$packageRoot.zip"
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archive
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$archive.sha256", "$hash  $(Split-Path $archive -Leaf)`n", (New-Object Text.UTF8Encoding($false)))
Write-Output "Package: $archive"
Write-Output "SHA-256: $hash"

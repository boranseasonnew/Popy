param([Parameter(Mandatory=$true)][string]$Config, [switch]$NoRestart)
$ErrorActionPreference = 'Stop'
$data = Get-Content -LiteralPath $Config -Raw -Encoding UTF8 | ConvertFrom-Json
$root = [IO.Path]::GetFullPath($data.root).TrimEnd('\')
$staged = [IO.Path]::GetFullPath($data.staged).TrimEnd('\')
$rootPrefix = $root + '\'
$stagedPrefix = $staged + '\'
$backup = Join-Path $root ('.tools\updates\backup-' + [guid]::NewGuid().ToString('N'))
$journal = New-Object Collections.Generic.List[object]
function Resolve-Child([string]$Base,[string]$Prefix,[string]$Relative) {
    if ([IO.Path]::IsPathRooted($Relative)) { throw 'Absolute update path rejected' }
    $path = [IO.Path]::GetFullPath((Join-Path $Base $Relative))
    if (!$path.StartsWith($Prefix,[StringComparison]::OrdinalIgnoreCase)) { throw 'Update path escapes its directory' }
    return $path
}
try {
    try { $parent = [Diagnostics.Process]::GetProcessById([int]$data.parent_pid) } catch { $parent = $null }
    if ($parent -and !$parent.WaitForExit(30000)) { throw 'App did not close; update was not installed' }
    New-Item -ItemType Directory -Path $backup -Force | Out-Null
    foreach ($relative in $data.files) {
        $source = Resolve-Child $staged $stagedPrefix $relative
        $destination = Resolve-Child $root $rootPrefix $relative
        $first = ($relative -replace '\\','/').Split('/')[0].ToLowerInvariant()
        if ($first -in @('.tools','exports','runs','fixtures')) { throw 'Update cannot replace user data' }
        $saved = Resolve-Child $backup ($backup + '\') $relative
        $existed = Test-Path -LiteralPath $destination -PathType Leaf
        if ($existed) {
            New-Item -ItemType Directory -Path (Split-Path $saved -Parent) -Force | Out-Null
            Copy-Item -LiteralPath $destination -Destination $saved
        }
        $journal.Add(@{destination=$destination;backup=$saved;existed=$existed})
        New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
        $temporary = $destination + '.popy-new'
        Copy-Item -LiteralPath $source -Destination $temporary -Force
        Move-Item -LiteralPath $temporary -Destination $destination -Force
    }
    $result = @{state='completed';version=$data.version}
} catch {
    $failure = $_.Exception.Message
    $rollbackErrors = @()
    for ($i=$journal.Count-1;$i -ge 0;$i--) {
        $item = $journal[$i]
        try {
            if ($item.existed) { Copy-Item -LiteralPath $item.backup -Destination $item.destination -Force }
            elseif (Test-Path -LiteralPath $item.destination -PathType Leaf) { Remove-Item -LiteralPath $item.destination -Force }
        } catch { $rollbackErrors += $_.Exception.Message }
    }
    $result = @{state='failed';error=$failure;rollback_errors=$rollbackErrors;backup=$backup}
}
$result['id'] = [guid]::NewGuid().ToString('N')
function Write-Result {
    $temporary = Join-Path $root '.tools\update-result.json.tmp'
    $destination = Join-Path $root '.tools\update-result.json'
    [IO.File]::WriteAllText($temporary,($result | ConvertTo-Json -Depth 5),(New-Object Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $temporary -Destination $destination -Force
}
Write-Result
if (!$NoRestart) {
    $restarted = Start-Process -FilePath (Join-Path $root 'popy-desktop.exe') -WorkingDirectory $root -WindowStyle Hidden -PassThru
    $result['restart_pid'] = $restarted.Id
    Write-Result
}

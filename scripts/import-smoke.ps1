param([string]$Cli = '', [Parameter(Mandatory=$true)][string]$ToolsConfig, [string]$RealInput = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if (!$Cli) { $Cli = Join-Path $projectRoot 'target\release\popy.exe' }
$Cli = [IO.Path]::GetFullPath($Cli)
$ToolsConfig = [IO.Path]::GetFullPath($ToolsConfig)
$tools = Get-Content -LiteralPath $ToolsConfig -Raw -Encoding UTF8 | ConvertFrom-Json
$ffmpeg = $tools.ffmpeg
if (![IO.Path]::IsPathRooted($ffmpeg)) { $ffmpeg = Join-Path (Split-Path $ToolsConfig -Parent) $ffmpeg }
$testRoot = Join-Path $projectRoot ('target\import-smoke\' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
$env:POPY_TOOLS = $ToolsConfig
$cases = @(
    @{name='hevc-10bit.mkv';args=@('-c:v','libx265','-x265-params','log-level=error:pools=1:frame-threads=1','-pix_fmt','yuv420p10le','-color_primaries','bt709','-color_trc','bt709','-colorspace','bt709')},
    @{name='hdr-pq.mkv';args=@('-c:v','libx265','-x265-params','log-level=error:pools=1:frame-threads=1:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc','-pix_fmt','yuv420p10le','-color_primaries','bt2020','-color_trc','smpte2084','-colorspace','bt2020nc')},
    @{name='hdr-hlg.mkv';args=@('-c:v','libx265','-x265-params','log-level=error:pools=1:frame-threads=1:colorprim=bt2020:transfer=arib-std-b67:colormatrix=bt2020nc','-pix_fmt','yuv420p10le','-color_primaries','bt2020','-color_trc','arib-std-b67','-colorspace','bt2020nc')},
    @{name='vp9-opus.webm';args=@('-itsoffset','0.2','-f','lavfi','-i','sine=sample_rate=48000:duration=0.3','-c:v','libvpx-vp9','-deadline','realtime','-cpu-used','8','-c:a','libopus')},
    @{name='mjpeg.avi';args=@('-c:v','mjpeg','-pix_fmt','yuvj422p')},
    @{name='rgb16.mkv';args=@('-c:v','ffv1','-pix_fmt','gbrp16le')},
    @{name='variable.mkv';args=@('-vf',"select='not(mod(n,3))+not(mod(n,5))'",'-fps_mode','vfr','-c:v','ffv1')},
    @{name='anamorphic.mkv';source='testsrc2=size=160x90:rate=25:duration=0.48';args=@('-vf','setsar=4/3,setfield=tff','-flags','+ilme+ildct','-c:v','mpeg2video')},
    @{name='animation.gif';args=@('-c:v','gif')},
    @{name='odd-rgb.mkv';source='testsrc=size=161x91:rate=12:duration=0.5';args=@('-c:v','ffv1','-pix_fmt','bgr0')}
)
$results = @()
foreach ($case in $cases) {
    $inputFile = Join-Path $testRoot $case.name
    $source = if ($case.source) { $case.source } else { 'testsrc2=size=160x90:rate=12:duration=0.5' }
    & $ffmpeg -hide_banner -loglevel error -nostdin -n -f lavfi -i $source @($case.args) $inputFile
    if ($LASTEXITCODE -ne 0) { throw "Fixture generation failed: $($case.name)" }
    $out = Join-Path $testRoot ($case.name + '-import')
    $json = & $Cli prepare --input $inputFile --out $out
    if ($LASTEXITCODE -ne 0) { throw "Import failed: $($case.name). See $out" }
    $report = ($json -join "`n") | ConvertFrom-Json
    if (!$report.source_unchanged -or $report.media.color_space -ne 'bt709' -or $report.media.frame_count -lt 1) { throw 'Invalid import report' }
    if ($case.name -like 'hdr-*' -and !$report.conversion.hdr_to_sdr) { throw 'HDR conversion was not recorded' }
    if ($case.name -eq 'vp9-opus.webm' -and !$report.media.has_audio) { throw 'Audio was lost' }
    if ($case.name -eq 'odd-rgb.mkv' -and ($report.media.width -ne 162 -or $report.media.height -ne 92)) { throw 'Odd dimensions were not padded correctly' }
    $results += @{name=$case.name;passed=$true;frames=$report.media.frame_count;audio=$report.media.has_audio}
    Write-Output "PASS $($case.name)"
}
if ($RealInput) {
    & $Cli prepare --input $RealInput --out (Join-Path $testRoot 'real-input') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Provided real-file import failed' }
    $results += @{name='provided-real-input';passed=$true}
}
[IO.File]::WriteAllText((Join-Path $testRoot 'summary.json'),($results | ConvertTo-Json -Depth 5),(New-Object Text.UTF8Encoding($false)))
Write-Output "Import smoke checks passed: $testRoot"

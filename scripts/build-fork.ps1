#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateRange(1, 4)][int]$Jobs = 2,
    [string]$RuntimeDirectory
)

# Keep compilation from competing with dictation on the same computer.
$ErrorActionPreference = 'Stop'
$buildProcess = [Diagnostics.Process]::GetCurrentProcess()
$previousPriority = $buildProcess.PriorityClass
$previousAffinity = $buildProcess.ProcessorAffinity
$previousJobs = $env:CARGO_BUILD_JOBS
$previousRuntime = $env:TRANSCRIBE_DIR
$previousRedist = $env:HANDY_VC_REDIST_DIRS
$repo = Split-Path -Parent $PSScriptRoot
$buildExit = 0
Push-Location -LiteralPath $repo
try {
    $buildProcess.PriorityClass = 'Idle'
    if ([Environment]::ProcessorCount -gt 4) { $buildProcess.ProcessorAffinity = [IntPtr]15 }
    $env:CARGO_BUILD_JOBS = [string]$Jobs
    $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH

    if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere)) { throw 'Install the C++ Build Tools described in BUILD.md.' }
        $vsPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if (-not $vsPath) { throw 'No C++ Build Tools installation found.' }
        $vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
        $compilerEnvironment = & cmd.exe /c "`"$vcvars`" >nul && set"
        foreach ($line in $compilerEnvironment) {
            if ($line -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process') }
        }
    }
    if (-not $env:VULKAN_SDK) {
        $env:VULKAN_SDK = [Environment]::GetEnvironmentVariable('VULKAN_SDK', 'User')
        if (-not $env:VULKAN_SDK) { $env:VULKAN_SDK = [Environment]::GetEnvironmentVariable('VULKAN_SDK', 'Machine') }
    }
    if ($env:VULKAN_SDK) { $env:PATH = (Join-Path $env:VULKAN_SDK 'Bin') + ';' + $env:PATH }
    if (-not $env:HANDY_VC_REDIST_DIRS -and $env:VCINSTALLDIR) {
        $redist = Get-ChildItem -Path (Join-Path $env:VCINSTALLDIR 'Redist\MSVC\*\x64\Microsoft.VC143.CRT') -Directory | Sort-Object FullName -Descending | Select-Object -First 1
        if ($redist) { $env:HANDY_VC_REDIST_DIRS = $redist.FullName }
    }
    if ($RuntimeDirectory) {
        if (-not (Test-Path -LiteralPath (Join-Path $RuntimeDirectory 'lib\transcribe-link.json'))) { throw 'RuntimeDirectory must be a transcribe.cpp install prefix.' }
        $env:TRANSCRIBE_DIR = (Resolve-Path -LiteralPath $RuntimeDirectory).Path
    }
    foreach ($tool in @('bun', 'cargo', 'cmake', 'glslc')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing $tool. See BUILD.md." }
    }
    & bun run tauri build --bundles nsis
    $buildExit = $LASTEXITCODE
} finally {
    $env:CARGO_BUILD_JOBS = $previousJobs
    $env:TRANSCRIBE_DIR = $previousRuntime
    $env:HANDY_VC_REDIST_DIRS = $previousRedist
    $buildProcess.ProcessorAffinity = $previousAffinity
    $buildProcess.PriorityClass = $previousPriority
    Pop-Location
}
if ($buildExit -ne 0) { throw "Fork build failed with exit code $buildExit" }

[CmdletBinding()]
param(
    [switch]$SkipCommandParity
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $repositoryRoot
try {
    if ([string]::IsNullOrWhiteSpace($env:WGPU_BACKEND)) {
        $env:WGPU_BACKEND = 'vulkan'
    }

    & cargo test --locked -p opencad-desktop --test desktop_smoke --features occt -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw 'Desktop backend smoke test failed'
    }

    if (-not $SkipCommandParity) {
        & cargo test --locked -p opencad-desktop --test command_parity
        if ($LASTEXITCODE -ne 0) {
            throw 'Desktop command parity audit failed'
        }
    }
}
finally {
    Pop-Location
}

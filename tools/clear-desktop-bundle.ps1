param(
    [Parameter(Mandatory = $true)][string]$BundleDirectory
)

$ErrorActionPreference = "Stop"

$Root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$TargetRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $Root "apps/desktop/src-tauri/target")
)
$Bundle = if ([System.IO.Path]::IsPathRooted($BundleDirectory)) {
    [System.IO.Path]::GetFullPath($BundleDirectory)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $Root $BundleDirectory))
}

$comparison = if ($IsWindows) {
    [System.StringComparison]::OrdinalIgnoreCase
} else {
    [System.StringComparison]::Ordinal
}
$targetPrefix = $TargetRoot.TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
) + [System.IO.Path]::DirectorySeparatorChar
if (-not $Bundle.StartsWith($targetPrefix, $comparison)) {
    throw "Desktop bundle cleanup must stay inside $TargetRoot; got $Bundle"
}
if ([System.IO.Path]::GetFileName($Bundle) -ne "bundle" -or
    [System.IO.Path]::GetFileName([System.IO.Path]::GetDirectoryName($Bundle)) -ne "release") {
    throw "Desktop bundle cleanup target must end with release/bundle: $Bundle"
}

if (Test-Path -LiteralPath $Bundle) {
    if (-not (Test-Path -LiteralPath $Bundle -PathType Container)) {
        throw "Desktop bundle cleanup target is not a directory: $Bundle"
    }
    Remove-Item -LiteralPath $Bundle -Recurse -Force
}
Write-Host "Desktop bundle output is clean: $Bundle"

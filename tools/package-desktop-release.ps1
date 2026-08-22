param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Platform,
    [Parameter(Mandatory = $true)][string]$BundleDirectory,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

if ($Version -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
    throw "Invalid desktop version: $Version"
}
if ($Platform -notmatch '^(linux|windows|macos)-(x86_64|aarch64)$') {
    throw "Invalid desktop platform: $Platform"
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Bundle = if ([System.IO.Path]::IsPathRooted($BundleDirectory)) {
    [System.IO.Path]::GetFullPath($BundleDirectory)
} else {
    Join-Path $Root $BundleDirectory
}
if (-not (Test-Path -LiteralPath $Bundle -PathType Container)) {
    throw "Tauri bundle directory does not exist: $Bundle"
}

$Output = if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $Root $OutputDirectory))
}
if (Test-Path -LiteralPath $Output) {
    $existing = @(Get-ChildItem -LiteralPath $Output -Force)
    if ($existing.Count -ne 0) {
        throw "Desktop release output directory must be empty: $Output"
    }
} else {
    New-Item -ItemType Directory -Force -Path $Output | Out-Null
}

$BundleFiles = @(Get-ChildItem -LiteralPath $Bundle -Recurse -File)
$ArtifactNames = [System.Collections.Generic.List[string]]::new()

function Add-FileArtifact {
    param(
        [Parameter(Mandatory = $true)][System.IO.FileInfo[]]$Candidates,
        [Parameter(Mandatory = $true)][string]$DestinationName
    )

    if ($Candidates.Count -ne 1) {
        $found = ($Candidates | ForEach-Object FullName) -join ", "
        throw "Expected exactly one bundle file for $DestinationName; found $($Candidates.Count): $found"
    }

    $destination = Join-Path $Output $DestinationName
    Copy-Item -LiteralPath $Candidates[0].FullName -Destination $destination
    if (-not (Test-Path -LiteralPath $destination -PathType Leaf) -or
        (Get-Item -LiteralPath $destination).Length -eq 0) {
        throw "Bundle artifact was not created: $destination"
    }
    $ArtifactNames.Add($DestinationName)
}

switch -Regex ($Platform) {
    '^windows-' {
        $msi = @($BundleFiles | Where-Object { $_.Extension -ieq ".msi" })
        $nsis = @($BundleFiles | Where-Object {
                $_.Extension -ieq ".exe" -and $_.Name -match '(?i)-setup\.exe$'
            })
        Add-FileArtifact $msi "musubicad-v$Version-$Platform.msi"
        Add-FileArtifact $nsis "musubicad-v$Version-$Platform-nsis.exe"
        break
    }
    '^linux-' {
        $deb = @($BundleFiles | Where-Object { $_.Extension -ieq ".deb" })
        $appImage = @($BundleFiles | Where-Object { $_.Extension -ieq ".appimage" })
        Add-FileArtifact $deb "musubicad-v$Version-$Platform.deb"
        Add-FileArtifact $appImage "musubicad-v$Version-$Platform.AppImage"
        break
    }
    '^macos-' {
        $dmg = @($BundleFiles | Where-Object { $_.Extension -ieq ".dmg" })
        Add-FileArtifact $dmg "musubicad-v$Version-$Platform.dmg"

        $apps = @(Get-ChildItem -LiteralPath $Bundle -Recurse -Directory | Where-Object {
                $_.Name.EndsWith(".app", [System.StringComparison]::OrdinalIgnoreCase)
            })
        if ($apps.Count -ne 1) {
            $found = ($apps | ForEach-Object FullName) -join ", "
            throw "Expected exactly one macOS app bundle; found $($apps.Count): $found"
        }

        $appArchiveName = "musubicad-v$Version-$Platform-app.zip"
        $appArchive = Join-Path $Output $appArchiveName
        & ditto -c -k --sequesterRsrc --keepParent $apps[0].FullName $appArchive
        if ($LASTEXITCODE -ne 0 -or
            -not (Test-Path -LiteralPath $appArchive -PathType Leaf) -or
            (Get-Item -LiteralPath $appArchive).Length -eq 0) {
            throw "macOS app archive was not created: $appArchive"
        }
        $ArtifactNames.Add($appArchiveName)
        break
    }
    default {
        throw "Unsupported desktop platform: $Platform"
    }
}

$expectedCount = if ($Platform.StartsWith("macos-")) { 2 } else { 2 }
if ($ArtifactNames.Count -ne $expectedCount) {
    throw "Expected $expectedCount desktop artifacts, found $($ArtifactNames.Count)"
}

$checksumLines = @(
    foreach ($name in ($ArtifactNames | Sort-Object)) {
        $path = Join-Path $Output $name
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        "$hash  $name"
    }
)
$checksumPath = Join-Path $Output "SHA256SUMS"
$utf8NoBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false
[System.IO.File]::WriteAllText(
    $checksumPath,
    (($checksumLines -join [Environment]::NewLine) + [Environment]::NewLine),
    $utf8NoBom
)

$checksumEntries = @(Get-Content -LiteralPath $checksumPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if ($checksumEntries.Count -ne $ArtifactNames.Count) {
    throw "SHA256SUMS does not contain one entry per desktop artifact"
}
$checksumNames = [System.Collections.Generic.List[string]]::new()
foreach ($line in $checksumEntries) {
    $match = [regex]::Match($line, '^([0-9a-fA-F]{64})  (.+)$')
    if (-not $match.Success) {
        throw "Invalid SHA256SUMS entry: $line"
    }
    $name = $match.Groups[2].Value
    if (-not ($ArtifactNames -contains $name)) {
        throw "SHA256SUMS references an unexpected file: $name"
    }
    $checksumNames.Add($name)
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $Output $name)).Hash
    if ($actual -ine $match.Groups[1].Value) {
        throw "SHA-256 verification failed for $name"
    }
}
$uniqueChecksumNames = @($checksumNames | Sort-Object -Unique)
if ($uniqueChecksumNames.Count -ne $ArtifactNames.Count) {
    throw "SHA256SUMS must reference every artifact exactly once"
}
foreach ($name in $ArtifactNames) {
    if (-not ($checksumNames -contains $name)) {
        throw "SHA256SUMS does not cover artifact: $name"
    }
}

Write-Host "Created desktop artifacts in $Output"
foreach ($name in ($ArtifactNames | Sort-Object)) {
    Write-Host "  $name"
}
Write-Host "  SHA256SUMS"

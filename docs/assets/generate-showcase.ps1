# Regenerate the flagship robot-joint GIFs from the committed Design Graph.
$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

Push-Location $Root
try {
    cargo run --locked -p opencad-cli -- animate-features `
        examples/robot_joint_actuator.ocad.d `
        docs/assets/robot-joint-feature-build.gif `
        --frames 54 --fps 9 --orbit-deg 35 --pitch-deg 30
    if ($LASTEXITCODE -ne 0) {
        throw "Feature-build GIF generation failed with exit code $LASTEXITCODE"
    }

    cargo run --locked -p opencad-cli -- animate `
        examples/robot_joint_actuator.ocad.d `
        docs/assets/robot-joint-orbit.gif `
        --frames 60 --fps 12 --orbit-deg 360 --pitch-deg 28
    if ($LASTEXITCODE -ne 0) {
        throw "Orbit GIF generation failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

Write-Host "Regenerated robot-joint Feature-build and 360-degree orbit GIFs."

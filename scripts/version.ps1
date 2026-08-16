$ErrorActionPreference = "Stop"

# Semver: vX.Y.Z, vX.Y.Z-alpha.N, vX.Y.Z-beta.N, vX.Y.Z-rc.N
$tagPattern = '^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-(alpha|beta|rc)\.(0|[1-9]\d*))?$'

function Get-CargoVersion {
    $match = Select-String -Path (Join-Path $PSScriptRoot "../Cargo.toml") -Pattern '^version = "(.+)"'
    if (-not $match) {
        throw "version not found in Cargo.toml"
    }
    return $match.Matches[0].Groups[1].Value
}

$isTag = $env:GITHUB_REF_TYPE -eq "tag"
if ($isTag) {
    $tag = $env:GITHUB_REF_NAME
    if ($tag -notmatch $tagPattern) {
        throw "Unsupported tag '$tag'. Use vX.Y.Z, vX.Y.Z-alpha.N, vX.Y.Z-beta.N, or vX.Y.Z-rc.N."
    }
    $version = $tag.Substring(1)
    $prerelease = ($tag -match '-(alpha|beta|rc)\.') 

    $cargoToml = Resolve-Path (Join-Path $PSScriptRoot "../Cargo.toml")
    $content = Get-Content -Raw -Path $cargoToml
    $updated = [regex]::Replace($content, '(?m)^version = "[^"]+"', "version = `"$version`"", 1)
    if ($updated -eq $content) {
        throw "failed to set Cargo.toml version to $version"
    }
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($cargoToml, $updated, $utf8)
    Push-Location (Join-Path $PSScriptRoot "..")
    try {
        cargo generate-lockfile
    }
    finally {
        Pop-Location
    }
} else {
    $version = Get-CargoVersion
    $prerelease = $false
}

Write-Host "version=$version prerelease=$prerelease"
if ($env:GITHUB_OUTPUT) {
    "version=$version" | Add-Content -Path $env:GITHUB_OUTPUT
    "prerelease=$($prerelease.ToString().ToLower())" | Add-Content -Path $env:GITHUB_OUTPUT
}

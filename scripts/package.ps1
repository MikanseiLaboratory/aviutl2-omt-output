param(
    [string]$Version = $env:RELEASE_VERSION
)

$ErrorActionPreference = "Stop"

if (-not $Version) {
    $Version = (Select-String -Path "$PSScriptRoot/../Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
}
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$dist = Join-Path $root "dist"
$stage = Join-Path $dist "stage"
$dll = Join-Path $root "target/x86_64-pc-windows-msvc/release/aviutl2_omt_live_output.dll"

if (-not (Test-Path $dll)) {
    throw "Release DLL not found: $dll"
}

if (Test-Path $stage) {
    Remove-Item -Recurse -Force $stage
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "Plugin/aviutl2_omt_live_output") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "Language") | Out-Null

Copy-Item $dll (Join-Path $stage "Plugin/aviutl2_omt_live_output.aux2")
Copy-Item (Join-Path $root "i18n/English.aviutl2_omt_live_output.aul2") (Join-Path $stage "Language/English.aviutl2_omt_live_output.aul2")
Copy-Item (Join-Path $root "i18n/Japanese.aviutl2_omt_live_output.aul2") (Join-Path $stage "Language/Japanese.aviutl2_omt_live_output.aul2")
Copy-Item (Join-Path $root "LICENSE") (Join-Path $stage "Plugin/aviutl2_omt_live_output/LICENSE")
Copy-Item (Join-Path $root "THIRD_PARTY_NOTICES.md") (Join-Path $stage "Plugin/aviutl2_omt_live_output/THIRD_PARTY_NOTICES.md")
Copy-Item (Join-Path $root "README.md") (Join-Path $stage "Plugin/aviutl2_omt_live_output/README.md")

@"
id=aviutl2-omt-output
name=AviUtl2 OMT Live Output
information=AviUtl2 OMT Live Output v$Version / 未完成成果物研究所
"@ | Set-Content -Encoding UTF8 (Join-Path $stage "package.ini")

$zipName = "aviutl2-omt-output-v$Version.au2pkg.zip"
$zipPath = Join-Path $dist $zipName
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zipPath
Write-Host "Wrote $zipPath"

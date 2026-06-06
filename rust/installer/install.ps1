# Gemma Genie — OS-agnostic install prelude (Windows).
#   iwr -useb https://raw.githubusercontent.com/sbmandava/gemma-genie/main/rust/installer/install.ps1 | iex
$ErrorActionPreference = "Stop"
$base = if ($env:GENIE_BOOTSTRAP_BASE) { $env:GENIE_BOOTSTRAP_BASE } else { "https://github.com/sbmandava/gemma-genie/releases/latest/download" }
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { "x86_64" }
$bin = "genie-bootstrap-$arch-windows.exe"
$tmp = Join-Path $env:TEMP $bin
Write-Host "Fetching $bin ..."
Invoke-WebRequest -UseBasicParsing -Uri "$base/$bin" -OutFile $tmp
$manifest = if ($env:GENIE_MANIFEST) { $env:GENIE_MANIFEST } else { "$base/manifest.json" }
& $tmp --manifest $manifest --install @args

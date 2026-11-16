# Genera los manifiestos de winget de una versión ya publicada en GitHub.
#
# Descarga SHA256SUMS.txt del release, lee la suma de orbyt.exe y escribe los tres archivos que
# pide microsoft/winget-pkgs (versión, instalador y textos) en dist/winget/<versión>/. No envía
# nada: el pull request a winget-pkgs se hace a mano (o con wingetcreate) después de revisarlos.
#
#   pwsh scripts/generar-winget.ps1 -Version 0.2.0

param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [string]$Repositorio = 'UtilityForCoders/focus',
    [string]$Identificador = 'UtilityForCoders.Orbyt',
    [string]$Editor = 'UtilityForCoders',
    # Suma SHA-256 de orbyt.exe. Si no se da, se lee del release.
    [string]$Sha256 = ''
)

$ErrorActionPreference = 'Stop'
$base = "https://github.com/$Repositorio/releases/download/v$Version"

if (-not $Sha256) {
    $sumas = (Invoke-WebRequest -Uri "$base/SHA256SUMS.txt" -UseBasicParsing).Content
    if ($sumas -is [byte[]]) { $sumas = [Text.Encoding]::UTF8.GetString($sumas) }
    $linea = $sumas -split "`n" | Where-Object { $_ -match 'orbyt\.exe' } | Select-Object -First 1
    if (-not $linea) { throw "SHA256SUMS.txt del release v$Version no nombra orbyt.exe." }
    $Sha256 = ($linea.Trim() -split '\s+')[0]
}
if ($Sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "La suma SHA-256 no es válida: $Sha256" }
$Sha256 = $Sha256.ToUpper()

$carpeta = Join-Path 'dist/winget' $Version
New-Item -ItemType Directory -Force $carpeta | Out-Null
$esquema = '1.6.0'

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.$esquema.schema.json
PackageIdentifier: $Identificador
PackageVersion: $Version
DefaultLocale: es-ES
ManifestType: version
ManifestVersion: $esquema
"@ | Set-Content (Join-Path $carpeta "$Identificador.yaml") -Encoding utf8

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.$esquema.schema.json
PackageIdentifier: $Identificador
PackageVersion: $Version
MinimumOSVersion: 10.0.17134.0
InstallerType: portable
Commands:
- orbyt
Installers:
- Architecture: x64
  InstallerUrl: $base/orbyt.exe
  InstallerSha256: $Sha256
ManifestType: installer
ManifestVersion: $esquema
"@ | Set-Content (Join-Path $carpeta "$Identificador.installer.yaml") -Encoding utf8

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.$esquema.schema.json
PackageIdentifier: $Identificador
PackageVersion: $Version
PackageLocale: es-ES
Publisher: $Editor
PublisherUrl: https://github.com/$Editor
PackageName: Orbyt
PackageUrl: https://github.com/$Repositorio
License: MIT
LicenseUrl: https://github.com/$Repositorio/blob/main/LICENSE
ShortDescription: Lanzador radial para Windows. Mantén el botón central del ratón, apunta y suelta.
Tags:
- lanzador
- launcher
- radial-menu
- productividad
ReleaseNotesUrl: https://github.com/$Repositorio/releases/tag/v$Version
ManifestType: defaultLocale
ManifestVersion: $esquema
"@ | Set-Content (Join-Path $carpeta "$Identificador.locale.es-ES.yaml") -Encoding utf8

"Manifiestos de $Identificador $Version en $carpeta"

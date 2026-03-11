param(
  [Parameter(Mandatory = $false)]
  [string]$Version = "latest",

  [Parameter(Mandatory = $false)]
  [string]$InstallDir,

  [Parameter(Mandatory = $false)]
  [string]$BinDir
)

$ErrorActionPreference = "Stop"
$repo = if ($env:EARL_INSTALL_REPO) { $env:EARL_INSTALL_REPO } else { "mathematic-inc/earl" }

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($arch) {
  "X64" { $target = "x86_64-pc-windows-msvc" }
  "Arm64" { $target = "aarch64-pc-windows-msvc" }
  default { throw "Unsupported architecture: $arch" }
}

if ($Version -eq "latest") {
  $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
  if (-not $latest.tag_name) {
    throw "Failed to resolve latest release tag"
  }
  $Version = $latest.tag_name.TrimStart("v")
}

$Version = $Version.TrimStart("v")
$tag = "v$Version"
$fileName = "earl-$Version-$target.zip"

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("earl-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
  $archivePath = Join-Path $tempDir $fileName
  $checksumsPath = Join-Path $tempDir "SHA256SUMS"

  $archiveUrl = "https://github.com/$repo/releases/download/$tag/$fileName"
  $checksumsUrl = "https://github.com/$repo/releases/download/$tag/SHA256SUMS"

  Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath
  Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath

  $checksumLine = Get-Content $checksumsPath | Where-Object { $_ -match "\s$([regex]::Escape($fileName))$" } | Select-Object -First 1
  if (-not $checksumLine) {
    throw "Checksum entry for $fileName was not found"
  }

  $expectedHash = ($checksumLine -split "\s+")[0].ToLowerInvariant()
  $actualHash = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($expectedHash -ne $actualHash) {
    throw "Checksum verification failed for $fileName"
  }

  $extractPath = Join-Path $tempDir "extract"
  Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force

  if ($BinDir) {
    $destinationDir = $BinDir
  } elseif ($InstallDir) {
    $destinationDir = Join-Path $InstallDir "bin"
  } else {
    $destinationDir = Join-Path $env:LOCALAPPDATA "Programs\earl\bin"
  }

  New-Item -ItemType Directory -Path $destinationDir -Force | Out-Null
  Copy-Item -Path (Join-Path $extractPath "earl.exe") -Destination (Join-Path $destinationDir "earl.exe") -Force

  Write-Host "Installed earl $Version to $destinationDir\earl.exe"

  $pathParts = $env:PATH -split ';'
  if ($pathParts -notcontains $destinationDir) {
    Write-Host "Add $destinationDir to PATH to run 'earl' from any shell."
  }
}
finally {
  if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
  }
}

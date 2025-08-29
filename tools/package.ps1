# Build and package script for packetmock project
param(
    [string]$Executable = "packetmock.exe",
    [string]$OutputDir = "dist",
    [string]$OutputZip = "release.zip"
)

try {
    # Navigate to project root (parent directory of tools)
    $ScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Definition
    $RootPath = Split-Path -Parent $ScriptDirectory

    Write-Host "Navigating to project root: $RootPath" -ForegroundColor Green
    Set-Location $RootPath

    # Download WinDivert
    Write-Host "`nRunning WinDivert script..." -ForegroundColor Green
    $windirvertScript = ".\tools\windivert.ps1"
    if (-not (Test-Path $windirvertScript)) {
        Write-Error "WinDivert script not found at $windirvertScript"
        exit 1
    }
    & $windirvertScript
    if ($LASTEXITCODE -ne 0) {
        Write-Error "WinDivert download failed"
        exit 1
    }

    # Run cargo fmt check
    Write-Host "`nChecking code formatting..." -ForegroundColor Green
    cargo fmt -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Code formatting check failed. Run 'cargo fmt' to fix formatting."
        exit 1
    }
    Write-Host "Code formatting check passed ✅" -ForegroundColor Green

    # Run cargo clippy
    Write-Host "`nRunning Clippy..." -ForegroundColor Green
    cargo clippy --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Clippy check failed"
        exit 1
    }
    Write-Host "Clippy check passed ✅" -ForegroundColor Green

    # Build release
    Write-Host "`nBuilding release..." -ForegroundColor Green
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Release build failed"
        exit 1
    }
    Write-Host "Release build completed ✅" -ForegroundColor Green

    # Determine architecture
    Write-Host "`nDetermining architecture..." -ForegroundColor Green
    $Architecture = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
    Write-Host "Detected architecture: $Architecture" -ForegroundColor Cyan
	$SysSuffix = if ($Architecture -eq "x64") { "64" } else { "32" }

    # Check required files
    $ExeFile = "target\release\$Executable"
    $WinDivertDll = "windivert\$Architecture\WinDivert.dll"
    $WinDivertSys = "windivert\$Architecture\WinDivert$SysSuffix.sys"

    $RequiredFiles = @($ExeFile, $WinDivertDll, $WinDivertSys)

    foreach ($file in $RequiredFiles) {
        if (-not (Test-Path $file)) {
            Write-Error "Required file not found: $file"
            exit 1
        }
    }
    Write-Host "All required files found ✅" -ForegroundColor Green

    # Create package directory
    $PackageDir = "package-temp"
    if (Test-Path $PackageDir) {
        Remove-Item -Path $PackageDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null

    # Copy files to package directory
    Write-Host "`nPackaging files..." -ForegroundColor Green
    foreach ($file in $RequiredFiles) {
        $fileName = Split-Path $file -Leaf
        Copy-Item -Path $file -Destination (Join-Path $PackageDir $fileName)
        Write-Host "  📄 Copied $fileName" -ForegroundColor White
    }

    # Create output directory
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    }

    # Create zip file
    Write-Host "`nCreating zip file..." -ForegroundColor Green
    $ZipPath = Join-Path $OutputDir $OutputZip
    if (Test-Path $ZipPath) {
        Remove-Item -Path $ZipPath -Force
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($PackageDir, $ZipPath)

    # Clean up package directory
    Remove-Item -Path $PackageDir -Recurse -Force

    # Get zip file info
    $ZipInfo = Get-Item $ZipPath
    Write-Host "`nPackaging completed! ✅" -ForegroundColor Green
    Write-Host "Package created: $($ZipInfo.FullName)" -ForegroundColor Cyan
    Write-Host "Package size: $([math]::Round($ZipInfo.Length / 1MB, 2)) MB" -ForegroundColor Cyan

    # List zip contents
    Write-Host "`nPackage contents:" -ForegroundColor Cyan
    Add-Type -AssemblyName System.IO.Compression
    $zip = [System.IO.Compression.ZipFile]::OpenRead($ZipInfo.FullName)
    try {
        $zip.Entries | ForEach-Object {
            $size = if ($_.Length -gt 0) { " ($([math]::Round($_.Length / 1KB, 1)) KB)" } else { "" }
            Write-Host "  📄 $($_.Name)$size" -ForegroundColor White
        }
    }
    finally {
        $zip.Dispose()
    }
}
catch {
    Write-Error "An error occurred: $($_.Exception.Message)"
    exit 1
}
finally {
    # Clean up temporary files
    if (Test-Path "package-temp") {
        Remove-Item -Path "package-temp" -Recurse -Force
    }
    Write-Host "`nBuild and package process completed." -ForegroundColor Gray

		exit 0
}

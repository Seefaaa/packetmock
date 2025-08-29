# WinDivert download and extraction script
param(
    [string]$DownloadUrl = "https://github.com/basil00/WinDivert/releases/download/v2.2.2/WinDivert-2.2.2-A.zip",
    [string]$ExtractPath = "../windivert"
)

# Get the script's directory and resolve the extract path relative to it
$ScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ResolvedExtractPath = Join-Path $ScriptDirectory $ExtractPath | Resolve-Path -ErrorAction SilentlyContinue
if (-not $ResolvedExtractPath) {
    $ResolvedExtractPath = [System.IO.Path]::GetFullPath((Join-Path $ScriptDirectory $ExtractPath))
}

# Temporary file path
$TempZipPath = [System.IO.Path]::GetTempFileName() + ".zip"

try {
    Write-Host "Downloading WinDivert..." -ForegroundColor Green
    Write-Host "URL: $DownloadUrl"

    # Download the file
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZipPath -UseBasicParsing
    Write-Host "Download completed." -ForegroundColor Green

    # Create target directory (clean if exists)
    if (Test-Path $ResolvedExtractPath) {
        Write-Host "Cleaning existing directory: $ResolvedExtractPath" -ForegroundColor Yellow
        Remove-Item -Path $ResolvedExtractPath -Recurse -Force
    }
    New-Item -ItemType Directory -Path $ResolvedExtractPath -Force | Out-Null

    Write-Host "Extracting zip file..." -ForegroundColor Green

    # Create a temporary extraction directory
    $TempExtractPath = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TempExtractPath -Force | Out-Null

    # .NET Framework to extract zip to temporary location
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($TempZipPath, $TempExtractPath)

    # Find the first subdirectory (WinDivert-2.2.2-A)
    $SubDirectory = Get-ChildItem -Path $TempExtractPath -Directory | Select-Object -First 1

    if ($SubDirectory) {
        Write-Host "Found subdirectory: $($SubDirectory.Name)" -ForegroundColor Cyan

        # Move contents from subdirectory to target path
        Get-ChildItem -Path $SubDirectory.FullName -Recurse | ForEach-Object {
            $DestinationPath = $_.FullName.Replace($SubDirectory.FullName, $ResolvedExtractPath)
            if ($_.PSIsContainer) {
                New-Item -ItemType Directory -Path $DestinationPath -Force | Out-Null
            } else {
                $DestinationDir = Split-Path -Path $DestinationPath -Parent
                if (-not (Test-Path $DestinationDir)) {
                    New-Item -ItemType Directory -Path $DestinationDir -Force | Out-Null
                }
                Copy-Item -Path $_.FullName -Destination $DestinationPath -Force
            }
        }

        # Clean up temporary extraction directory
        Remove-Item -Path $TempExtractPath -Recurse -Force
    } else {
        Write-Error "No subdirectory found in the zip file"
        exit 1
    }

    Write-Host "Extraction completed!" -ForegroundColor Green
    Write-Host "Files extracted to: $ResolvedExtractPath" -ForegroundColor Cyan

    # List extracted files
    Write-Host "`nExtracted files:" -ForegroundColor Cyan
    Get-ChildItem -Path $ResolvedExtractPath -Recurse | ForEach-Object {
        $relativePath = $_.FullName.Substring($ResolvedExtractPath.Length + 1)
        if ($_.PSIsContainer) {
            Write-Host "  📁 $relativePath" -ForegroundColor Blue
        } else {
            Write-Host "  📄 $relativePath" -ForegroundColor White
        }
    }
}
catch {
    Write-Error "An error occurred: $($_.Exception.Message)"
    exit 1
}
finally {
    # Clean up temporary files
    if (Test-Path $TempZipPath) {
        Remove-Item -Path $TempZipPath -Force
    }
    if (Test-Path $TempExtractPath) {
        Remove-Item -Path $TempExtractPath -Recurse -Force
    }
    Write-Host "`nTemporary files cleaned up." -ForegroundColor Gray
}

Write-Host "`nOperation completed successfully! ✅" -ForegroundColor Green

exit 0

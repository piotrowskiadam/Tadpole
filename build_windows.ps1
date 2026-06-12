# Windows Build Trigger Script for MSYS2

$msysPath = "C:\msys64"
if (-not (Test-Path $msysPath)) {
    Write-Error "MSYS2 was not found at $msysPath. Please install MSYS2 from https://www.msys2.org/ and retry."
    Exit 1
}

$bashExe = Join-Path $msysPath "usr\bin\bash.exe"
if (-not (Test-Path $bashExe)) {
    Write-Error "Bash executable was not found in MSYS2 at $bashExe."
    Exit 1
}

Write-Host "=== Starting Tadpole Windows Build via MSYS2 ===" -ForegroundColor Green

# Invoke the bash script with MSYS2 environment (login shell so paths are set up)
$proc = Start-Process -FilePath $bashExe -ArgumentList "-lc", "'./build_windows.sh'" -NoNewWindow -PassThru -Wait

if ($proc.ExitCode -ne 0) {
    Write-Error "Build failed with exit code $($proc.ExitCode)"
    Exit $proc.ExitCode
}

Write-Host "=== Windows Build Successfully Completed! ===" -ForegroundColor Green
Write-Host "Output package is located at: dist/tadpole-windows.zip" -ForegroundColor Cyan

# This script takes care of packaging the build artifacts that will go in the
# release zipfile

$SRC_DIR = $PWD.Path
$STAGE = [System.Guid]::NewGuid().ToString()

Set-Location $ENV:Temp
New-Item -Type Directory -Name $STAGE
Set-Location $STAGE

$ZIP = "$SRC_DIR\$($Env:CRATE_NAME)-$($Env:VERSION)-$($Env:TARGET).zip"
$ZIP_SHA256 = "$SRC_DIR\$($Env:CRATE_NAME)-$($Env:VERSION)-$($Env:TARGET).zip.sha256"

# TODO Update this to package the right artifacts
Copy-Item "$SRC_DIR\target\$($Env:TARGET)\release\svmdev.exe" "$SRC_DIR\svmdev-win-$($Env:VERSION).exe"

7z a "$ZIP" *
Get-FileHash "$ZIP" -Algorithm SHA256 | Out-File "$ZIP_SHA256"

Push-AppveyorArtifact "$ZIP"
Push-AppveyorArtifact "$ZIP_SHA256"

Remove-Item *.* -Force
Set-Location ..
Remove-Item $STAGE
Set-Location $SRC_DIR

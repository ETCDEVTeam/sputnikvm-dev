echo "Rust version is $($env:RUST_VERSION)"

If ($env:APPVEYOR_REPO_BRANCH -eq "master" -AND $env:RUST_VERSION -eq "stable") {
    echo "Deploy to http://builds.etcdevteam.com/sputnikvm-dev/$env:VERSION_BASE/"
    janus.exe deploy -to="builds.etcdevteam.com/sputnikvm-dev/$env:VERSION_BASE/" -files=".\janus\*.zip" -key=".\.ci\.gcloud.json"
}
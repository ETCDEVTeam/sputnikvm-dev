If ($env:APPVEYOR_REPO_BRANCH -eq 'gcp-deploy') {
    echo "Deploy to http://builds.etcdevteam.com/sputnikvm-dev/$env:VERSION_BASE/"
    janus.exe deploy -to="builds.etcdevteam.com/sputnikvm-dev/$env:VERSION_BASE/" -files=".\janus\*.zip" -key=".\.ci\.gcloud.json"
}
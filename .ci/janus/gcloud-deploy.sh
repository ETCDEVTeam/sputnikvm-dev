#!/usr/bin/env bash

set -e

VERSION_BASE=$(janus version -format='v%M.%m.x')
echo "Deploy to http://builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/"

janus deploy -to="builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/" -files="./janus/*" -key=".ci/.gcloud.json"

echo "Deployed"

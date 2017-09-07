#!/usr/bin/env bash

set -e

if [ "$TRAVIS_BRANCH" -eq "gce-deploy" ]; then
    if [ "$TRAVIS_RUST_VERSION" -eq "stable" ]; then
        VERSION_BASE=$(janus version -format='v%M.%m.x')
        echo "Deploy to http://builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/"

        janus deploy -to="builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/" -files="./janus/*" -key=".ci/.gcloud.json"

        echo "Deployed"
    fi
fi

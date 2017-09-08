#!/usr/bin/env bash

set -e

export PATH=$PATH:$(pwd)/janusbin
echo "Check condition branch: $TRAVIS_BRANCH rust version: $TRAVIS_RUST_VERSION"

if [ "$TRAVIS_PULL_REQUEST" = "false" ]; then
    if [ "$TRAVIS_BRANCH" = "master" ]; then
        if [ "$TRAVIS_RUST_VERSION" = "stable" ]; then
            VERSION_BASE=$(janus version -format='v%M.%m.x')
            echo "Deploy to http://builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/"

            janus deploy -to="builds.etcdevteam.com/sputnikvm-dev/$VERSION_BASE/" -files="./janus/*" -key=".ci/.gcloud.json"

            echo "Deployed"
        fi
    fi
fi

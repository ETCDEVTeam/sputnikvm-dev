# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    openssl aes-256-cbc -k "$GCP_PASSWD" -in .ci/janus/gcloud-travis.json.enc -out .ci/.gcloud.json -d

    curl -sL https://raw.githubusercontent.com/ethereumproject/janus/master/get.sh | bash
    export PATH=$PATH:$PWD/janusbin
    export APP_VERSION_GIT_TAG="$(janus version -format 'v%M.%m.%P-%C-%S')"

    cargo build --release --all

    cp target/release/svmdev $stage/

    cd $stage
    mkdir -p $src/janus
    tar czf $src/janus/svmdev-$TRAVIS_OS_NAME-$APP_VERSION_GIT_TAG.tar.gz *
    cd $src

    rm -rf $stage
}

main

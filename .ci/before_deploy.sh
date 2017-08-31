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

    cargo build --release --all

    cp target/release/svmdev $stage/

    cd $stage
    tar czf $src/svmdev-$TRAVIS_OS_NAME-$TRAVIS_TAG.tar.gz *
    shasum -a 256 $src/svmdev-$TRAVIS_OS_NAME-$TRAVIS_TAG.tar.gz
    shasum -a 256 $src/svmdev-$TRAVIS_OS_NAME-$TRAVIS_TAG.tar.gz > $src/svmdev-$TRAVIS_OS_NAME-$TRAVIS_TAG.tar.gz.sha256
    cd $src

    rm -rf $stage
}

main

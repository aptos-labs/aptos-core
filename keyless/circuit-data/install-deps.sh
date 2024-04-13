#!/bin/sh

install_git_lfs() {
    if which brew > /dev/null; then
        brew install git-lfs
    elif which apt-get > /dev/null; then
        sudo apt-get install git-lfs
    else
        echo "Can't figure out what platform you are on. Currently this script only supports MacOS and Debian."
    fi
    git lfs install
}

install_circom() {
    echo -n "Checking if $1 is installed..."
    if which $1 > /dev/null; then
        echo "yes."
    else
        echo "no, installing..."

        cd `mktemp -d`
        git clone http://github.com/iden3/circom
        cd circom
        git switch -d v2.1.7
        cargo build --release
        cargo install --path circom
        cd ..
    fi
}

install_pip3_deps() {
  pip3 install virtualenv pyjwt cryptography pycryptodome
}
  
install_npm_deps() {
    npm install -g snarkjs
}

install_pip3_deps
install_npm_deps
install_circom
install_git_lfs

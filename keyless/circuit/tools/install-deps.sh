#!/bin/sh

set -e
install_node() {
    echo "nodejs installation started."
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.1/install.sh | bash
    export NVM_DIR="$HOME/.nvm"
    [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm
    [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"  # This loads nvm bash_completion
    nvm install --lts
    node -v
    npm -v
    echo "nodejs installation finished."
}

install_git_lfs() {
    echo "git lfs installation started."
    if which brew > /dev/null; then
        brew install git-lfs
    elif which apt-get > /dev/null; then
        sudo apt-get install git-lfs
    else
        echo "Can't figure out what platform you are on. Currently this script only supports MacOS and Debian."
    fi
    git lfs install
    echo "git lfs installation finished."
}

install_circom() {
    echo "circom installation started."
    original_dir=$(pwd)
    cd `mktemp -d`
    git clone https://github.com/iden3/circom
    cd circom
    git switch -d v2.1.7
    cargo build --release
    cargo install --path circom
    cd "$original_dir"
    echo "circom installation finished."
}

install_pip3_deps() {
    echo "pip3 deps installation started."
    sudo apt install python3-venv -y
    echo "pip3 deps installation finished."
}

install_npm_deps() {
    echo "snarkjs installation started."
    npm install -g snarkjs
    echo "snarkjs installation finished."
}

install_circomlib() {
    echo "circomlib@2.0.5 installation started."
    npm install -g circomlib@2.0.5
    echo "circomlib@2.0.5 installation finished."
}

install_node
install_pip3_deps
install_npm_deps
install_circomlib
install_circom
install_git_lfs

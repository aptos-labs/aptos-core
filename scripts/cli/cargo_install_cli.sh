#/bin/sh
# Check if version is provided
if [ -z "$1" ]; then
    echo "Error: Version argument is required"
    exit 1
fi

export VERSION=$1

# Check if version matches expected format
if ! echo "$VERSION" | grep -q "^[0-9]\+\.[0-9]\+\.[0-9]\+$"; then
    echo "Error: Version must be in format X.X.X"
    exit 1
fi

RUSTFLAGS="--cfg tokio_unstable" cargo install --locked --git https://github.com/velor-chain/velor-core.git --profile cli velor --tag velor-cli-v$VERSION
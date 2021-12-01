# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

echo Installing dev dependencies
sh ${DIEM_HOME}/scripts/dev_setup.sh -yp
echo Installing Move CLI
pushd ${DIEM_HOME}
cargo build -p df-cli
popd
echo Move CLI installed

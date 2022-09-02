#!/bin/bash

# This script syncs the grafana dashboards from the grafana server to the local copy of this repo.
# It is intended to be run from the root of the repo.

### Install grafana-sync tool
if [[ "$(uname)" == "Darwin" ]]; then
    wget https://github.com/mpostument/grafana-sync/releases/download/1.4.8/grafana-sync_1.4.8_Darwin_x86_64.tar.gz
    sha=$(shasum -a 256 grafana-sync_1.4.8_Darwin_x86_64.tar.gz | awk '{ print $1 }')
    [ "$sha" != "64be888acf049dea9485f002ee38e5a597f35a9b9ed7913cfbfd163747694c2c" ] && echo "shasum mismatch" && exit 1
    tar -xvf grafana-sync_1.4.8_Darwin_x86_64.tar.gz grafana-sync
else # Assume Linux
    wget https://github.com/mpostument/grafana-sync/releases/download/1.4.8/grafana-sync_1.4.8_Linux_x86_64.tar.gz
    sha=$(shasum -a 256 grafana-sync_1.4.8_Linux_x86_64.tar.gz | awk '{ print $1 }')
    [ "$sha" != "c1b5a2c0d2b081d8acffaa06aebc83bca7cd47fdc8f3e7b4c252952b4fe15ec0" ] && echo "shasum mismatch" && exit 1
    tar -xvf grafana-sync_1.4.8_Linux_x86_64.tar.gz grafana-sync
fi
chmod +x grafana-sync

## Pull dashboards from grafana from the "aptos-core" folder
./grafana-sync pull-dashboards --apikey="${GRAFANA_API_KEY}" --directory="dashboards" --url https://o11y.aptosdev.com --folderName="aptos-core"
ret=$?
if [ $ret -ne 0 ]; then
    echo "Failed to pull dashboards from grafana"
    exit $ret
fi

## Reformat dashboards to be more readable
npx --yes prettier@2.7.1 --write ./dashboards

## Compress
gzip -fkn dashboards/*.json

## Check dashboards changes in the dashboards directory
git status dashboards

## Clean up local resources
rm grafana-sync*

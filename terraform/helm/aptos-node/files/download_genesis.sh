#!/bin/sh
set -x
cp /opt/aptos/genesis_readonly/* /opt/aptos/genesis
signed_url_cf_base_url="https://us-west1-aptos-forge-gcp-0.cloudfunctions.net/signed-url"
signed_url=$(curl -s -X GET "$signed_url_cf_base_url?namespace=${NAMESPACE}&era=${ERA}&method=GET")
curl -o /opt/aptos/genesis/genesis.blob "$signed_url"
ls -l /opt/aptos/genesis

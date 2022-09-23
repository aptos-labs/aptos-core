#!/bin/bash

if [ -z "$GITHUB_TOKEN" ]; then
  echo "Github token not found"
  exit 1
fi

response=$(curl -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${GITHUB_TOKEN}" https://api.github.com/repos/aptos-labs/wallet/releases)

first_assets_url=`echo $response | jq -r .[0].assets_url`
name=`echo $response | jq -r .[0].name`

echo $first_assets_url
echo $name
#!/bin/bash

if [ -z "$GITHUB_TOKEN" ]; then
  echo "Github token not found"
  exit 1
fi

echo "Getting all releases..."
response=$(curl -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${GITHUB_TOKEN}" https://api.github.com/repos/aptos-labs/wallet/releases)

upload_url=`echo $response | jq -r .[0].upload_url`
name=`echo $response | jq -r .[0].name`
id=`echo $response | jq -r .[0].id`

echo "Upload url: ${upload_url}"
echo "Release name: ${name}"
echo "Release ID: ${id}"

# Return first_upload_url and name
echo "::set-output name=upload_url::${upload_url}"
echo "::set-output name=name::${name}"

# If it's a draft, is_draft === true
if [[ $name =~ "Draft" ]]; then
  echo "Last release was a draft release, deleting previous release..."
  delete_response = $(curl -X DELETE -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${GITHUB_TOKEN}" https://api.github.com/repos/aptos-labs/wallet/releases/$id)
  echo "Deletion complete"
  echo "::set-output name=is_draft::true"
else
  echo "Last release was not a draft release, not deleting previous release..."
  echo "::set-output name=is_draft::false"
fi

# Aptos Python SDK Changelog

All notable changes to the Aptos Python SDK will be captured in this file. This changelog is written by hand for now.

## 0.7.0
- Delete sync client
- Port remaining sync examples to async (hello-blockchain, multisig, your-coin)
- Updated token client to use events to acquire minted tokens
- Update many dependencies and set Python 3.8.1 as the minimum requirement
- Add support for an experimental chunked uploader

## 0.6.4
- Change sync client library from httpX to requests due to latency concerns.

## 0.6.2
- Added custom header "x-aptos-client" to both sync/async RestClient

## 0.6.1
- Updated package manifest.

## 0.6.0
- Add token client.
- Add support for generating account addresses.
- Add support for http2
- Add async client


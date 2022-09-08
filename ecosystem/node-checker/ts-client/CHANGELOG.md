# Aptos Node Health Checker TS client changelog

All notable changes to the client will be captured in this file. This changelog is written by hand for now. It adheres to the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

**Note:** The client does not follow semantic version while we are in active development. Instead, breaking changes will be announced with each devnet cut. Once we launch our mainnet, the SDK will follow semantic versioning closely.

## 0.0.2 (2022-09-01)
- Added `public_key` as an optional field to `check_node`. If an evaluator that needs it, e.g. the `HandshakeEvaluator`, is configured as part of the baseline config, it will return an error indicating as such if it is not provided.

## 0.0.1 (2022-08-31)
- Initial release.

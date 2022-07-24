# Changelog

All notable changes to this project will be documented in this file.

## 1.3.0 (2022-07-23)

### Refactors

- [move] [Aptos Framework] Rename TestCoin to AptosCoin ([d02c86ed74](https://github.com/aptos-labs/aptos-core/commit/d02c86ed746999ddd984535e494fb45fb5806ec5))
- [move] Using new UTF8 strings instead of ASCII strings ([b189a79507](https://github.com/aptos-labs/aptos-core/commit/b189a795076cf78c799f4ecbc63c9527c2997170))
- [move] Upgrade Move step #2 ([7343c6453f](https://github.com/aptos-labs/aptos-core/commit/7343c6453fe1ec843f3e1839804b5ed0515b145b))
- [move] Upgrade to the newest state of the move-language repo ([28934df501](https://github.com/aptos-labs/aptos-core/commit/28934df501d4662d97bb6ef5486ace87c94b96b3))
- [devtools] common prettier format for all JS/TS files ([e168e83d6e](https://github.com/aptos-labs/aptos-core/commit/e168e83d6ee9503aa39f3062413401e00ecb9109))

## 1.2.0 (2022-06-28)

### Features

- deprecate getTokenBalance api in SDK ([2ec554e](https://github.com/aptos-labs/aptos-core/commit/2ec554e6e40a81cee4e760f6f84ef7362c570240))
- memoize chain id in aptos client ([#1589](https://github.com/aptos-labs/aptos-core/issues/1589)) ([4a6453b](https://github.com/aptos-labs/aptos-core/commit/4a6453bf0e620247557854053b661446bff807a7))
- **mutiagent:** support multiagent transaction submission ([#1543](https://github.com/aptos-labs/aptos-core/issues/1543)) ([0f0c70e](https://github.com/aptos-labs/aptos-core/commit/0f0c70e8ed2fefa952f0c89b7edb78edc174cb49))
- support retrieving token balance for any account ([7f93c21](https://github.com/aptos-labs/aptos-core/commit/7f93c2100f8b8e848461a0b5a395bfb76ade8667))
- vector tests for transaction signing ([6210c10](https://github.com/aptos-labs/aptos-core/commit/6210c10d3192fd0417b35709545fae850099e4d4))
- add royalty support for NFT tokens ([93a2cd0](https://github.com/aptos-labs/aptos-core/commit/93a2cd0bfd644725ac524f419e94077e0b16343b))
- add transaction builder examples ([a710a50](https://github.com/aptos-labs/aptos-core/commit/a710a50e8177258d9c0766762b3c2959fc231259))
- support transaction simulation ([93073bf](https://github.com/aptos-labs/aptos-core/commit/93073bf1b508d00cfa1f8bb441ed57085fd08a82))

### Bug Fixes

- **ts-sdk:** fix a typo, natual now becomes natural ([1b7d295](https://github.com/aptos-labs/aptos-core/commit/1b7d2957b79a5d2821ada0c5096cf43c412e0c2d)), closes [#1526](https://github.com/aptos-labs/aptos-core/issues/1526)
- Fix Javascript example ([5781fee](https://github.com/aptos-labs/aptos-core/commit/5781fee74b8f2b065e7f04c2f76952026860751d)), closes [#1405](https://github.com/aptos-labs/aptos-core/issues/1405)

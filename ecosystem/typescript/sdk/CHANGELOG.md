# Changelog

All notable changes to this project will be documented in this file.

### 1.2.2 (2022-07-27)


### Features

* add ABI interfaces to aptos client ([1d0fe29](https://github.com/aptos-labs/aptos-core/commit/1d0fe29cc4d2c7b9bd1b19f95059e532ef4a4936))
* add abi support to TS SDK ([f0214b4](https://github.com/aptos-labs/aptos-core/commit/f0214b4deec4f3ab401782c7f4d793431f1f3e9c))
* add UserTransaction and hashing capability ([9168969](https://github.com/aptos-labs/aptos-core/commit/916896947c629fe02588d5c5977bff2fecf587ac))
* allow TransactionBuilder to build raw transactions with binary ABI ([fe0c325](https://github.com/aptos-labs/aptos-core/commit/fe0c325a3e853ba5bf809a3878a8edaa350a2068))
* deprecate getTokenBalance api in SDK ([2ec554e](https://github.com/aptos-labs/aptos-core/commit/2ec554e6e40a81cee4e760f6f84ef7362c570240))
* memoize chain id in aptos client ([#1589](https://github.com/aptos-labs/aptos-core/issues/1589)) ([4a6453b](https://github.com/aptos-labs/aptos-core/commit/4a6453bf0e620247557854053b661446bff807a7))
* **mutiagent:** support multiagent transaction submission ([#1543](https://github.com/aptos-labs/aptos-core/issues/1543)) ([0f0c70e](https://github.com/aptos-labs/aptos-core/commit/0f0c70e8ed2fefa952f0c89b7edb78edc174cb49))
* reimplement type tag parser ([67473a1](https://github.com/aptos-labs/aptos-core/commit/67473a1c35616733442a38055aa2c0440aa1315f))
* support retrieving token balance for any account ([7f93c21](https://github.com/aptos-labs/aptos-core/commit/7f93c2100f8b8e848461a0b5a395bfb76ade8667))
* **ts-sdk:** accepts string parameter as vec<u8> ([0daade4](https://github.com/aptos-labs/aptos-core/commit/0daade4f734d1ba29a896b00d7ddde2249e87970))
* **ts-sdk:** add a transaction builder that supports ABI ([95296a7](https://github.com/aptos-labs/aptos-core/commit/95296a7b75c5765214005054975b71d5d2215263))
* **ts-sdk:** e2e test for ABI interface ([edad199](https://github.com/aptos-labs/aptos-core/commit/edad1994b2e7501733256d20251e45b0d46646da))
* update move to latest version ([23a953b](https://github.com/aptos-labs/aptos-core/commit/23a953b3f1a222a71d496222f0dcd8ce17fc8cac))


### Bug Fixes

* get rid of "natual" calls ([#1678](https://github.com/aptos-labs/aptos-core/issues/1678)) ([54601f7](https://github.com/aptos-labs/aptos-core/commit/54601f79206ea0f8b8b1b0d6599d31832fc4d195))
* **ts-sdk:** fix a typo, natual now becomes natural ([1b7d295](https://github.com/aptos-labs/aptos-core/commit/1b7d2957b79a5d2821ada0c5096cf43c412e0c2d)), closes [#1526](https://github.com/aptos-labs/aptos-core/issues/1526)

### 1.2.1 (2022-07-23)


### Features

* deprecate getTokenBalance api in SDK ([2ec554e](https://github.com/aptos-labs/aptos-core/commit/2ec554e6e40a81cee4e760f6f84ef7362c570240))
* memoize chain id in aptos client ([#1589](https://github.com/aptos-labs/aptos-core/issues/1589)) ([4a6453b](https://github.com/aptos-labs/aptos-core/commit/4a6453bf0e620247557854053b661446bff807a7))
* **mutiagent:** support multiagent transaction submission ([#1543](https://github.com/aptos-labs/aptos-core/issues/1543)) ([0f0c70e](https://github.com/aptos-labs/aptos-core/commit/0f0c70e8ed2fefa952f0c89b7edb78edc174cb49))
* support retrieving token balance for any account ([7f93c21](https://github.com/aptos-labs/aptos-core/commit/7f93c2100f8b8e848461a0b5a395bfb76ade8667))


### Bug Fixes

* get rid of "natual" calls ([#1678](https://github.com/aptos-labs/aptos-core/issues/1678)) ([54601f7](https://github.com/aptos-labs/aptos-core/commit/54601f79206ea0f8b8b1b0d6599d31832fc4d195))
* **ts-sdk:** fix a typo, natual now becomes natural ([1b7d295](https://github.com/aptos-labs/aptos-core/commit/1b7d2957b79a5d2821ada0c5096cf43c412e0c2d)), closes [#1526](https://github.com/aptos-labs/aptos-core/issues/1526)

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

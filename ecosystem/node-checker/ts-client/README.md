# Generated TS client for Velor Node Health Checker

[![Discord][discord-image]][discord-url]
[![NPM Package Version][npm-image-version]][npm-url]
[![NPM Package Downloads][npm-image-downloads]][npm-url]

## Quickstart
```bash
pnpm add velor-node-checker-client
```

You can also use `yarn` or `npm`.

## Usage

For Javascript or Typescript usage, check out the [`./examples`][examples] folder with ready-made `package.json` files to get you going quickly!

If you are using the types in a `commonjs` module, like in a Node app, you just have to enable `esModuleInterop`
and `allowSyntheticDefaultImports` in your `tsconfig` for types compatibility:

```json
{
  ...
  "compilerOptions": {
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true
    ...
  }
}
```

### Requirements

- [Node.js](https://nodejs.org)
- [pnpm](https://pnpm.io/installation)

```bash
pnpm install
```

### Generating API client
To generate the client from the spec, run:

```bash
pnpm generate-client
```

### Running tests
Run a local node (run from the root of the repo):
```
cargo run -p velor -- node run-local-testnet --faucet-port 8081 --force-restart --assume-yes
```

Run a local Node Health Checker:
```
cargo run -p velor-node-checker -- server run --baseline-node-config-paths ecosystem/node-checker/configuration_examples/local_testnet.yaml --listen-address 0.0.0.0
```

Run the tests:
```
pnpm test
```

If you see strange behavior regarding HTTP clients, try running the tests with `--detectOpenHandles`.

## Semantic versioning

This project follows [semver](https://semver.org/) as closely as possible.

## Release process

To release a new version of the SDK do the following.

1. Regenerate the client:
```
pnpm generate-client
```

2. Test:
```
pnpm test
```

3. Bump the version in `package.json` according to [semver](https://semver.org/).
4. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
5. Once you're confident everything is correct, submit your PR.
6. Once the PR is approved and on main, run `pnpm checked-publish` manually.

## References

[repo]: https://github.com/velor-chain/velor-core
[npm-image-version]: https://img.shields.io/npm/v/velor.svg
[npm-image-downloads]: https://img.shields.io/npm/dm/velor.svg
[npm-url]: https://npmjs.org/package/velor-node-checker-client
[discord-image]: https://img.shields.io/discord/945856774056083548?label=Discord&logo=discord&style=flat~~~~
[discord-url]: https://discord.gg/velornetwork

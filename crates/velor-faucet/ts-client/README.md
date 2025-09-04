# Generated TS client for the Velor Faucet

## Quickstart
```bash
pnpm add @velor-chain/velor-faucet-client
```

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
- [pnpm](https://pnpm.io/)

```bash
pnpm install
```

### Generating API client
To generate the client from the spec, run:

```bash
pnpm generate-client
```

### Running tests
Run a faucet locally. See the [README](../README.md) in the root for information on how to do that.

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


# SDK for Aptos Node API

[![Discord][discord-image]][discord-url]
[![NPM Package Version][npm-image-version]][npm-url]
[![NPM Package Downloads][npm-image-downloads]][npm-url]

## Quickstart

The public SDK downloaded from [npmjs](https://www.npmjs.com/package/aptos) is compatible with the [Aptos devnet](https://fullnode.devnet.aptoslabs.com). To start building, run below command in your project directory:

```bash
pnpm add aptos
```

or use the browser bundle

```
<script src="https://unpkg.com/aptos@latest/dist/index.global.js" />
```

Then, the SDK can be accessed through `window.aptosSDK`.

Learn the basics of how to use the SDK by following [this tutorial](https://aptos.dev/tutorials/your-first-transaction-sdk) in the Aptos developer site.

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
- [Yarn](https://pnpmpkg.com/)

```bash
pnpm install
```

### Generating API client

This SDK is composed of two parts, a core client generated from the OpenAPI spec of the API, and a set of wrappers that make it nicer to use, enable certain content types, etc.

To generate the core client from the spec, run:

```bash
pnpm generate-client
```

### Working with devnet

See the quickstart above.

### Working with local node

To develop in a local environment, you need to use the SDK from the [main](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) branch.

**NOTE**
SDK from the main branch might not be compatible with the devnet.

Run a local node (run from the root of the repo):

```
cargo run -p aptos -- node run-local-testnet --with-faucet --faucet-port 8081 --force-restart --assume-yes
```

Run the SDK tests and make sure they pass. Go to the SDK directory, and setup an env to configure the URLs:

```
rm .env
echo 'APTOS_NODE_URL="http://127.0.0.1:8080/v1"' >> .env
echo 'APTOS_FAUCET_URL="http://127.0.0.1:8081"' >> .env
```

Run the tests:

```
pnpm test
```

If you see strange behavior regarding HTTP clients, try running the tests with `--detectOpenHandles`.

Package the SDK and start building:

```bash
pnpm build
pnpm pack
# In your project directory
pnpm add PATH_TO_LOCAL_SDK_PACKAGE
```

## Semantic versioning

This project follows [semver](https://semver.org/) as closely as possible.

## Release process

To release a new version of the SDK do the following.

1. Check that the commit you're deploying from (likely just the latest commit of `main`) is green ln CI. Go to GitHub and make sure there is a green tick, specifically for the `sdk-release` release CI step. This ensures that the all tests, formatters, and linters passed, including server / client compatibility tests (within that commit) and tests to ensure the API, API spec, and client were all generated and match up.
2. Bump the version in `package.json` according to [semver](https://semver.org/).
3. Bump the version in `version.ts`
4. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Generally this means changing the "Unreleased" section to a version and then making a new "Unreleased" section.
5. Once you're confident everything is correct, submit your PR. The CI will ensure that you have followed all the previous steps, specifically ensuring that the API, API spec, and SDK client are all compatible, that you've updated the changelog, that the tests pass, etc.
6. Land the PR into the main branch. Make sure this commit comes up green in CI too.
7. Check out the latest commit on main.
8. Get the auth token from our password manager. Search for "npmjs". It should look like similar to this: `npm_cccaCVg0bWaaR741D5Gdsd12T4JpQre444aaaa`.
9. Run `pnpm publish --dry-run`. From here, make some sanity checks:
   a. Look closely at the output of the command. {ay close attention to what is packaged. Make sure we're not including some files that were included accidentally. For example `.aptos`. Add those to .npmignore if needed.
   b. Compare the summary with the public npm package summary on npmjs. The number of files and sizes should not vary too much.
10. Run `NODE_AUTH_TOKEN=<token> pnpm checked-publish`
11. Double check that the release worked by visitng npmjs: https://www.npmjs.com/package/aptos

[examples]: https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/
[repo]: https://github.com/aptos-labs/aptos-core
[npm-image-version]: https://img.shields.io/npm/v/aptos.svg
[npm-image-downloads]: https://img.shields.io/npm/dm/aptos.svg
[npm-url]: https://npmjs.org/package/aptos
[discord-image]: https://img.shields.io/discord/945856774056083548?label=Discord&logo=discord&style=flat~~~~
[discord-url]: https://discord.gg/aptoslabs
[api-doc]: https://aptos-labs.github.io/ts-sdk-doc/

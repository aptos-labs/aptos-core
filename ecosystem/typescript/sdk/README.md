# SDK for Aptos Node API

> **_NOTE:_**
> This is the `legacy TypeScript SDK`, aka the npm package `aptos`. For a more robust SDK and better support, we recommend upgrading to the `new TypeScript SDK` [@aptos-labs/ts-sdk](https://github.com/aptos-labs/aptos-ts-sdk). Take a look at the [documentation](https://aptos.dev/sdks/new-ts-sdk/) and the [migration guide](https://aptos.dev/sdks/new-ts-sdk/migration-guide).

[![Discord][discord-image]][discord-url]
[![NPM Package Version][npm-image-version]][npm-url]
[![NPM Package Downloads][npm-image-downloads]][npm-url]

The Aptos TypeScript SDK provides a convenient way to interact with the Aptos blockchain using TypeScript. It offers a set of utility functions, classes, and types to simplify the integration process and enhance developer productivity.

## Installation

##### For use in Node.js or a web application

```ts
pnpm install aptos
```

You can also use your preferred npm client, such as yarn or npm.

##### For use in a browser

```ts
<script src="https://unpkg.com/aptos@latest/dist/index.global.js" />
```

Then, the SDK can be accessed through `window.aptosSDK`.

## Documentation and examples

- [The Aptos documentation site](https://aptos.dev/sdks/ts-sdk/index) provides step-by-step instructions, code snippets, and best practices to use this library.
- You can view the generated [Type Doc](https://aptos-labs.github.io/ts-sdk-doc/) for the latest release of the SDK.
- For in-depth examples, check out the [examples](./examples) folder with ready-made `package.json` files to get you going quickly!

### Development environment setup

Setup an `.env` file to configure the URLs.
From the [root](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) of this package, run:

```ts
rm .env
echo 'APTOS_NODE_URL="http://localhost:8080/v1"' >> .env
echo 'APTOS_FAUCET_URL="http://localhost:8081"' >> .env
```

### Testing

To run the full SDK tests, From the [root](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) of this package, run:

```ts
pnpm test
```

> If you see strange behavior regarding HTTP clients, try running the tests with `--detectOpenHandles`.

To test a single file in the SDK, From the [root](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) of this package, run:

```ts
npx jest -- <path/to/file.test.ts>
```

To use the local build in a local project:

```ts
// run from the root of this package
pnpm build
// run on your local project
pnpm add PATH_TO_LOCAL_SDK_PACKAGE
```

### Working with local node

To develop in a local environment, you need to use the SDK from the [main](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) branch.

Run a local node (run from the root of the [repo](https://github.com/aptos-labs/aptos-core/)):

```ts
cargo run -p aptos -- node run-local-testnet --force-restart --assume-yes
```

## Contributing

If you found a bug or would like to request a feature, please file an [issue](https://github.com/aptos-labs/aptos-core/issues/new/choose). If, based on the discussion on an issue you would like to offer a code change, please make a [pull request](./CONTRIBUTING.md). If neither of these describes what you would like to contribute, checkout out the [contributing guide](./CONTRIBUTING.md).

[npm-image-version]: https://img.shields.io/npm/v/aptos.svg
[npm-image-downloads]: https://img.shields.io/npm/dm/aptos.svg
[npm-url]: https://npmjs.org/package/aptos
[discord-image]: https://img.shields.io/discord/945856774056083548?label=Discord&logo=discord&style=flat~~~~
[discord-url]: https://discord.gg/aptosnetwork

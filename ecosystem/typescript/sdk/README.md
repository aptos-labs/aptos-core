# Aptos TS/JS SDK

[![Discord][discord-image]][discord-url]
[![NPM Package Version][npm-image-version]][npm-url]
[![NPM Package Downloads][npm-image-downloads]][npm-url]

You need to connect to an [Aptos][repo] node to use this library, or run one
yourself locally.

## API Docs

Docs can be found [here][api-doc]

## Usage

For Javascript or Typescript usage, check out the [`./examples`][examples] folder with ready-made `package.json` files
to get you going quickly!

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
- [yarn](https://yarnpkg.com/)

```bash
sudo apt-get update
sudo apt-get install nodejs yarn
```

### Generating Types

Originally created with this:

```bash
$  npx swagger-typescript-api@latest -p ../../../api/doc/v0/openapi.yaml -o ./src/api --modular --axios --single-http-client
```

#### Changes to make after generation:

- OpenAPI/SpecHTML routes/types deleted as they're unneeded.
- There are a few type errors in the `http-client.ts` as the axios types are incomplete, that were fixed
  via `// @ts-ignore`

### Testing (jest)

```bash
yarn test
```

## Semantic versioning

This project follows [semver](https://semver.org/) as closely as possible

[examples]: https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/
[repo]: https://github.com/aptos-labs/aptos-core
[npm-image-version]: https://img.shields.io/npm/v/aptos.svg
[npm-image-downloads]: https://img.shields.io/npm/dm/aptos.svg
[npm-url]: https://npmjs.org/package/aptos
[discord-image]: https://img.shields.io/discord/945856774056083548?label=Discord&logo=discord&style=flat~~~~
[discord-url]: https://discord.gg/aptoslabs
[api-doc]: https://aptos-labs.github.io/ts-sdk-doc/

## Release process

1. Regenerate generated types `npx swagger-typescript-api@latest -p ../../../api/doc/v0/openapi.yaml -o ./src/api --modular --axios --single-http-client`
2. Update your commit message to follow the Angular Conventional Commits
3. Test lint and format `yarn test` `yarn lint` `yarn fmt`
4. Update the version in the `package.json` file and run `yarn changelog` to generate the changelog
5. Once it's all committed you can run npm release

Generate the change log. yarn changelog and bump the version in package.json. Only bump the minor version.
Commit the changes.
yarn build to build the package. Ideally, you should test the new package in browser to make sure it works in browser as well.
Follow https://aptos-org.slack.com/archives/C034HFWPJ05/p1651687417201819, to publish the package

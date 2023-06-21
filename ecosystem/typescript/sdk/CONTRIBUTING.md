# Contribution Guidelines for Typescript SDK

- Coding Styles
  - File names must use Snake case. For example, `aptos_account.ts` .
  - Class names must use Pascal case. For example, `class AuthenticationKey` .
  - Function and method names must use Camel case. For example, `derivedAddress(): HexString` .
  - Constants must use all caps (upper case) words separated by `_`. For example, `MAX_U8_NUMBER` .
- Comments
  - Comments are required for new classes and functions.
  - Comments should follow the TSDoc standard, [https://tsdoc.org/](https://tsdoc.org/).
- Lints and Formats
  - ESlint (eslint) and Prettier (prettier) should be used for code checking and code formatting. Make sure to run `pnpm lint` and `pnpm fmt` after making changes to the code.
- Tests
  - Unit tests are required for any non-trivial changes you make.
  - The Jest testing framework is used in the repo and we recommend you use it. See Jest: [https://jestjs.io/](https://jestjs.io/).
  - Make sure to run `pnpm test` after making changes.
- Commits
  - Commit messages follow the [Angular convention](https://www.conventionalcommits.org/en/v1.0.0-beta.4/#summary).

## Creating a pull request

You are welcome to create a pull request against the main branch.

Before creating a PR,

- Make sure your branch is up to date with the `main` branch.
- On the root folder, run `pnpm test`.
- On the root folder, run `pnpm fmt`.
- On the root folder, run `pnpm lint`.

If everything passes, you should be able to create a PR.

#### Changelog

This project keeps a changelog. If a pull request created needs to bump a package version, please follow those steps to create a changelog

1. Bump the version in `package.json` according to [semver](https://semver.org/).
2. Bump the version in `version.ts`.
3. Add the change description in the CHANGELOG under the "Unreleased" section.

## Release process

To release a new version of the SDK do the following.

1. Check that the commit you're deploying from (likely just the latest commit of `main`) is green in CI. Go to GitHub and make sure there is a green tick, specifically for the `sdk-release` release CI step. This ensures that the all tests, formatters, and linters passed, including server / client compatibility tests (within that commit) and tests to ensure the API, API spec, and client were all generated and match up.
2. Bump the version in `package.json` according to [semver](https://semver.org/).
3. Bump the version in `version.ts`.
4. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Generally this means changing the "Unreleased" section to a version and then making a new "Unreleased" section.
5. Once you're confident everything is correct, submit your PR. The CI will ensure that you have followed all the previous steps, specifically ensuring that the API, API spec, and SDK client are all compatible, that you've updated the changelog, that the tests pass, etc.
6. Land the PR into the main branch. Make sure this commit comes up green in CI too.
7. Check out the latest commit on main.
8. Get the auth token from our password manager. Search for "npmjs". It should look like similar to this: `npm_cccaCVg0bWaaR741D5Gdsd12T4JpQre444aaaa`.
9. Run `pnpm publish --dry-run`. From here, make some sanity checks:
   a. Look closely at the output of the command. {ay close attention to what is packaged. Make sure we're not including some files that were included accidentally. For example `.aptos`. Add those to .npmignore if needed.
   b. Compare the summary with the public npm package summary on npmjs. The number of files and sizes should not vary too much.
10. Run `NODE_AUTH_TOKEN=<token> pnpm checked-publish`.
11. Double check that the release worked by visitng npmjs: https://www.npmjs.com/package/aptos.

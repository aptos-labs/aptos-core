# Velor Protos

## Changelog
To update the changelog do the following:

1. Bump the version in `package.json` according to [semver](https://semver.org/).
1. Add the change description in the CHANGELOG under the "Unreleased" section.

## Release process
To release a new version of the package do the following.

1. Check that the commit you're deploying from (likely just the latest commit of `main`) is green in CI.
1. Bump the version in `package.json` according to [semver](https://semver.org/).
1. Bump the version in `version.ts`.
1. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Generally this means changing the "Unreleased" section to a version and then making a new "Unreleased" section.
1. Once the CI is green land the PR into the main branch.
1. Check out the latest commit on main.
1. Get the auth token from our password manager. Search for "npmjs". It should look like similar to this: `npm_cccaCVg0bWaaR741D5Gdsd12T4JpQre444aaaa`.
1. Run `pnpm publish --dry-run`. From here, make some sanity checks:
  a. Look closely at the output of the command. Pay close attention to what is packaged. Make sure we're not including some files that were included accidentally. For example `.velor`. Add those to .npmignore if needed.
  b. Compare the summary with the public npm package summary on npmjs. The number of files and sizes should not vary too much.
1. Run `pnpm build`
1. Run `NODE_AUTH_TOKEN=<token> pnpm publish --non-interactive`.
1. Double check that the release worked by visitng npmjs: https://www.npmjs.com/package/velor-protos.

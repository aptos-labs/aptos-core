# Aptos Protos

## Changelog
To update the changelog do the following:

1. Bump the version in `Cargo.toml` according to [semver](https://semver.org/).
1. Add the change description in the CHANGELOG under the "Unreleased" section.

## Release process
To release a new version of the package do the following.

1. Check that the commit you're deploying from (likely just the latest commit of `main`) is green in CI.
1. Bump the version in `Cargo.toml` according to [semver](https://semver.org/).
1. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Generally this means changing the "Unreleased" section to a version and then making a new "Unreleased" section.
1. Once the CI is green land the PR into the main branch.
1. Go to the Actions tab of the repo, click "Publish aptos-protos for Rust" and then "Run Workflow".
1. Double check that the release worked by visiting: https://crates.io/crates/aptos-protos.
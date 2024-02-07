# Aptos Protos

## Release process
To release a new version of the package do the following.

1. Check that the commit you're deploying from (likely just the latest commit of `main`) is green in CI.
1. Bump the version in `Cargo.toml` according to [semver](https://semver.org/).
1. Add an entry in the CHANGELOG for the version. We adhere to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Generally this means changing the "Unreleased" section to a version and then making a new "Unreleased" section.
1. Get the auth token from our password manager. Search for "crates io API token". Run `cargo login` and then paste the token in.
1. Run `cargo publish --dry-run` first just to make sure that the crate compiles happily.
1. Double check that the release worked by visitng crates.io: https://crates.io/crates/aptos-protos.

## Workspace
If you're using this crate from within aptos-core, consider relying on it via the workspace (`aptos-protos = { workspace = true }`). This is the simplest approach and makes iterating on development easier.

For other cases, e.g. when using this crate from other repos or crates in aptos-core have compatibility guarantees to uphold, you may choose to rely on the crate as release on crates.io instead.

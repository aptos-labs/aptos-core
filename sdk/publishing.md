# Publishing Runbook

When preparing to make a release of the SDK follow these steps:
1. Update version numbers of all crates to be published from `0.0.X` to `0.0.X + 1` (e.g., from `0.0.2` to `0.0.3`) including updating the version requirement for any packages which have
workspace dependencies which are also being published. 
2. Perform a dry-run publish (use `cargo publish --dry-run`) in order to verify that publishing will be successful.
3. Create a PR and get it merged into master.
4. Once the PR has landed in master, check out the commit which does the versions bump.
5. Publish to crates.io
6. Create a git tag `git tag velor-sdk-v0.0.X HEAD` and push that tag to the velor-chain/velor-core repository.

Here is the set of currently published packages that make up the velor-sdk:
* move-core-types
* velor-crypto-derive
* velor-crypto
* velor-types
* velor-json-rpc-types
* velor-client
* velor-sdk

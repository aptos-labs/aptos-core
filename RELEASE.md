# Aptos Release Process

## Branches and Tags

```
========================================= main branch ==========================================>
                           \                                  \                         \
                            \___aptos-node-v1.2.0 tag          \                         \
                             \                                  \                         \
                              \      aptos-framework-v1.3.0 tag__\                     devnet branch
   aptos-framework-v1.2.0 tag__\                                  \                     
                                \___aptos-node-v1.2.4 tag          \___aptos-node-v1.3.0 tag
                                 \                                  \
                                  \                                  \
                             aptos-release-v1.2 branch         aptos-release-v1.3 branch

```

### main branch
All current development occurs on the `main` branch. All new feature development
should use feature flag to gate it off during development, and turn it on once the
development is complete and passed the AIP process when it applies.

### devnet branch
The `devnet` branch is created of the `main` branch every week. It will be used to deploy
devnet and allow Aptos community to explore the most recent changes of the Aptos node binary
and Aptos framework.


### aptos-release-v*X.Y* release branches
These are release branches based on Aptos release planning timeline. They are created off
the `main` branch. Release branches are created on every 1-2 months cadence.

### aptos-node-v*X.Y.Z* release tag
The aptos node release tags are created for validator/fullnode deployment of the given release
branch. The minor number *Z* will increment when a new hot-fix release is required on this 
release branch. Aptos team will publish the matching tag docker images on 
[Aptos Docker Hub](https://hub.docker.com/r/aptoslabs/validator/tags) when available.

### aptos-framework-v*X.Y.Z* release tag
The aptos framework release tags are created to facilitate the on-chain framework upgrade of the 
given release branch. The minor number *Z* will increment when a new hot-fix release or a new 
framework update is required on this release branch.

### aptos-cli-v*X.Y.Z* release tag
The aptos cli release tags are created to track the CLI versions for community to use when
developing on the Aptos network. It's always recommended to upgrade your CLI when a new version
is released, for the best user experience.

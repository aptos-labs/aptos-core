# Docker Image Tagging

## Overview

Images are built into GCP Artifact Registry (internal), then copied to GCP and Docker Hub (public) during release. The tag format encodes the build profile, feature flags, and git ref as `_`-delimited segments — empty segments are dropped.

## Tag Anatomy

```
{IMAGE_TAG_PREFIX}[_{profile}][_{feature}][_{git_sha}]
```

| Segment | Value | When present |
|---|---|---|
| `IMAGE_TAG_PREFIX` | e.g. `aptos-node-v1.2.3`, `devnet`, `nightly` | Always |
| `profile` | `performance` | Only for non-`release` profiles |
| `feature` | e.g. `failpoints` | Only for non-`default` features |
| `git_sha` | full commit SHA | Always appended as a second immutable tag |

### Build profiles

| Profile | Tag segment |
|---|---|
| `release` (default) | _(omitted)_ |
| `performance` | `performance` |

### Build features

| Feature | Tag segment |
|---|---|
| `default` | _(omitted)_ |
| `failpoints` | `failpoints` |

Source: profile/feature prefix logic is implemented in [`docker/builder/docker-bake-rust-all.sh`](builder/docker-bake-rust-all.sh) and the `joinTagSegments` helper in [`docker/image-helpers.js`](image-helpers.js).

## Examples

| Scenario | Tag |
|---|---|
| release profile, default feature | `aptos-node-v1.2.3` |
| performance profile, default feature | `aptos-node-v1.2.3_performance` |
| release profile, failpoints feature | `aptos-node-v1.2.3_failpoints` |
| performance + failpoints | `aptos-node-v1.2.3_performance_failpoints` |

Each tag is also copied with the git SHA appended:
```
aptos-node-v1.2.3_performance → aptos-node-v1.2.3_performance_{git_sha}
```

## Source tags (GCP, pre-release)

Images are staged in GCP Artifact Registry during CI builds, tagged by profile/feature + git SHA:

```
{GCP_REPO}/{image}:[{profile}_][{feature}_]{git_sha}
```

Also tagged with the normalized branch/PR name for layer cache reuse:
```
{GCP_REPO}/{image}:[{profile}_][{feature}_]{branch_or_pr}
```

Examples:
- `validator:abc123`  ← release profile, default feature
- `validator:performance_abc123`  ← performance profile
- `validator:failpoints_abc123`  ← release + failpoints

Source: [`docker/builder/docker-bake-rust-all.hcl`](builder/docker-bake-rust-all.hcl) — the `generate_tags` function produces these tags for every image target.

## CI build workflows

Images are built by [`workflow-run-docker-rust-build.yaml`](../.github/workflows/workflow-run-docker-rust-build.yaml), which accepts `PROFILE` and `FEATURES` inputs and invokes `docker/builder/docker-bake-rust-all.sh`. It is called from [`docker-build-test.yaml`](../.github/workflows/docker-build-test.yaml) as three parallel jobs:

| Job | Profile | Features | Label to trigger on PRs |
|---|---|---|---|
| `rust-images` | `release` | — | always required |
| `rust-images-performance` | `performance` | — | `CICD:build-performance-images` |
| `rust-images-failpoints` | `release` | `failpoints` | `CICD:build-failpoints-images` |

`docker-build-test.yaml` also sets `PROFILE_RELEASE`, `PROFILE_PERF`, and `FEATURE_FAILPOINTS` flags that are forwarded to the wait-images step.

## Waiting for images

[`docker/wait-images-ci.mjs`](wait-images-ci.mjs) polls GCP for staged images before dependent CI jobs run. It is wrapped by the [`wait-images-ci` composite action](../.github/actions/wait-images-ci/action.yaml) which accepts the same three boolean flags (`PROFILE_RELEASE`, `PROFILE_PERF`, `FEATURE_FAILPOINTS`) to know which variant tags to wait for. The set of image+profile combinations it checks is defined by `getImagesToWaitFor` in [`docker/image-helpers.js`](image-helpers.js).

## Release workflows

### Versioned releases (`aptos-node-vX.Y.Z`, `aptos-indexer-grpc-vX.Y.Z`)

Triggered by pushing a tag or one of the named network branches (`devnet`, `testnet`, `mainnet`, etc.) via [`copy-images-to-dockerhub-release.yaml`](../.github/workflows/copy-images-to-dockerhub-release.yaml). The git ref name (`github.ref_name`) becomes `IMAGE_TAG_PREFIX`.

For `aptos-node-vX.Y.Z` tags, [`docker/image-helpers.js`](image-helpers.js) (`assertTagMatchesSourceVersion`) validates that the version in the tag matches `aptos-node/Cargo.toml` before copying. The release PR that bumps that version is created by [`aptos-node-release.yaml`](../.github/workflows/aptos-node-release.yaml).

### Nightly

[`copy-images-to-dockerhub-nightly.yaml`](../.github/workflows/copy-images-to-dockerhub-nightly.yaml) runs on dispatch with `IMAGE_TAG_PREFIX=nightly`.

### Core copy logic

Both release paths call [`copy-images-to-dockerhub.yaml`](../.github/workflows/copy-images-to-dockerhub.yaml), which runs [`docker/release-images.mjs`](release-images.mjs). That script:
1. Determines the release group from `IMAGE_TAG_PREFIX` (`getImageReleaseGroupByImageTagPrefix` in [`release-images.mjs`](release-images.mjs))
2. Iterates over the per-image release matrix (see below)
3. Copies `{GCP_REPO}/{image}:{profile}_{git_sha}` → `{registry}/{image}:{prefix}_{profile}` and also tags it with `_{git_sha}`

## Release groups

`IMAGE_TAG_PREFIX` selects which images are released together (defined in `IMAGES_TO_RELEASE_BY_RELEASE_GROUP` in [`docker/release-images.mjs`](release-images.mjs)):

| Prefix contains | Release group | Images |
|---|---|---|
| `aptos-node` (default) | `aptos-node` | `validator`, `validator-testing`, `faucet`, `tools` |
| `aptos-indexer-grpc` | `aptos-indexer-grpc` | `indexer-grpc` |

`validator-testing` is released to GCP only — never to Docker Hub (controlled by `IMAGE_NAMES_TO_RELEASE_ONLY_INTERNAL` in [`docker/release-images.mjs`](release-images.mjs)).

## Per-image release matrix

Each image declares which (profile, feature) combinations are released. Defined in `IMAGES_TO_RELEASE` in [`docker/release-images.mjs`](release-images.mjs):

| Image | Profiles |
|---|---|
| `validator` | `release`, `performance` |
| `validator-testing` | `release`, `performance` |
| `faucet` | `release`, `performance` |
| `tools` | `release`, `performance` |
| `indexer-grpc` | `release`, `performance` |

## Release validation

For `aptos-node-vX.Y.Z` prefixes, the script validates that `X.Y.Z` matches the version in `aptos-node/Cargo.toml` before copying. Non-release prefixes (e.g. `devnet`, `nightly`) skip this check. See `assertTagMatchesSourceVersion` and `isReleaseImage` in [`docker/image-helpers.js`](image-helpers.js).

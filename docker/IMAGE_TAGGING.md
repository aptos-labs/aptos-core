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
| `consensus-only-perf-test` | `consensus_only_perf_test` |

Source: the bake script gets its profile/feature prefix from [`docker/builder/image-tag-prefix.sh`](builder/image-tag-prefix.sh). The `joinTagSegments` helper in [`docker/image-helpers.js`](image-helpers.js) formats other image tags.

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
- `validator:consensus_only_perf_test_abc123`  ← release + consensus-only performance

Source: [`docker/builder/docker-bake-rust-all.hcl`](builder/docker-bake-rust-all.hcl) — the `generate_tags` function produces these tags for every image target.

## CI build workflows

Pull requests and trusted branch builds use physically separate dispatchers:

- [`docker-build-test.yaml`](../.github/workflows/docker-build-test.yaml) handles only `pull_request_target`. It computes one base-owned capability plan, then runs one non-fail-fast local-build matrix and one non-fail-fast protected-publication matrix. Repository identity, the exact source SHA, the base SHA, and the pull request number come directly from the event. Only the validated profile, feature, and build-target fields come from the matrix.
- [`docker-build-test-trusted.yaml`](../.github/workflows/docker-build-test-trusted.yaml) handles only pushes and manual dispatches. It retains the trusted generic build and Forge workflows, secret inheritance, and cloud publication path.

The reusable Docker build workflow forwards the requested profile/feature flags to the wait-images step and uses a separate build lock for each SHA/profile/feature combination.

The authoritative PR capability labels, capability identifiers, supported variants, and profile/feature mappings are defined only in [`.github/ci/docker-capabilities.json`](../.github/ci/docker-capabilities.json). The dispatcher does not duplicate this mapping. The supported variants are release, failpoints, performance, and consensus-only performance.

The nine stable branch-protection checks remain explicit jobs in the PR dispatcher. They consume only booleans from a base-owned status evaluator. The evaluator validates the complete plan against the authoritative manifest and fails closed if authorization, a required matrix, or an authorized workload does not succeed.

## Waiting for images

[`docker/wait-images-ci.mjs`](wait-images-ci.mjs) polls GCP for staged images before dependent CI jobs run. Its [`wait-images-ci` composite action](../.github/actions/wait-images-ci/action.yaml) accepts `PROFILE_RELEASE`, `PROFILE_PERF`, `FEATURE_FAILPOINTS`, and `FEATURE_CONSENSUS_ONLY`. The last flag requires `consensus_only_perf_test_<SHA>` tags; ordinary images cannot satisfy it. With no flags set, the historical release/performance/failpoints checks apply. The combinations are defined by `getImagesToWaitFor` in [`docker/image-helpers.js`](image-helpers.js).

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
| `aptos-node` (default) | `aptos-node` | `validator`, `validator-testing`, `faucet`, `tools`, `indexer-grpc` |
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

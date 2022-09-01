# Docker

This directory contains [Docker](https://www.docker.com/) configuration for building Aptos docker images.

To build these images run this from the repository root:

```
docker buildx create --use # creates a buildkit builder and only needs to be run once
docker/docker-bake-rust-all.sh
```

For using the images, look in the `compose` directory.

## Image tagging strategy

The `builder` target is the one that builds the rust binaries and is the most expensive. Its output is used by all the other targets that follow.

The `builder` itself takes in a few build arguments. Most are build metadata, such as `GIT_SHA` and `GIT_BRANCH`, but others change the build entirely, such as cargo flags `PROFILE` and `FEATURES`. Arguments like these necessitate a different cache to prevent clobbering. The general strategy is to use image tags and cache keys that use these variables. An example image tag might be:
* `performance_failpoints_<GIT_SHA>` -- `performance` profile with `failpoints` feature
* `<GIT_SHA>` -- default `release` profile with no additional features

# Docker

This directory contains [Docker](https://www.docker.com/) configuration for building Aptos docker images.

To build these images run this from the repository root:

```
docker buildx create --use # creates a buildkit builder and only needs to be run once
docker/docker-bake-rust-all.sh
```

For using the images, look in the `compose` directory.

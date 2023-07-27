# Use this Dockerfile to build a custom kind image with a specific kubernetes version
# Note the usage of --platform=arm64, to ensure it works on M1 Mac

# Somehow this doesn't work for 1.24, but works for 1.23
# FROM --platform=arm64 kindest/node:v1.24.13@sha256:c9e00e2b228e47ba3c96eaf0309b27dc3f73e444944e4c900016fd07b1b805cb

FROM --platform=arm64 kindest/node:v1.23.17@sha256:2dd8f45f8a00b559af35583b7c95ab0e289843b0dce1d4906bb6a2c0fe983056
RUN arch


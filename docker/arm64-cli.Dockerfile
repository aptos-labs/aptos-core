FROM  rust:1.66.1 as build
WORKDIR /app
COPY . .
RUN ./scripts/dev_setup.sh -b -r
RUN ./scripts/cli/build_cli_release.sh "Ubuntu-22.04"

FROM  ubuntu:22.04
COPY --from=build /app/aptos-cli-*.zip /
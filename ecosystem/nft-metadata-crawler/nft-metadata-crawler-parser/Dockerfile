FROM rust:1.70.0

USER root
ARG DATABASE_URL=postgres://docker:docker@localhost:5432/postgres
ENV DATABASE_URL=$DATABASE_URL

WORKDIR /nft-metadata-crawler-parser
COPY . .
COPY Cargo.docker.toml Cargo.toml

RUN cargo install --path /nft-metadata-crawler-parser
RUN cargo install diesel_cli --no-default-features --features postgres

CMD bash -c "diesel setup" && cargo run

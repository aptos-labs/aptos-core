FROM rust:1.70.0

ARG AUTH
ENV AUTH=$AUTH
ARG BUCKET
ENV BUCKET=$BUCKET
ARG SUBSCRIPTION_NAME
ENV SUBSCRIPTION_NAME=$SUBSCRIPTION_NAME
ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL

COPY nft-metadata-crawler-parser /nft-metadata-crawler-parser
COPY nft-metadata-crawler-utils /nft-metadata-crawler-utils

COPY nft-metadata-crawler-parser/Cargo.docker.toml /nft-metadata-crawler-parser/Cargo.toml
COPY nft-metadata-crawler-utils/Cargo.docker.toml /nft-metadata-crawler-utils/Cargo.toml

RUN cd nft-metadata-crawler-parser && cargo build --release

CMD cd nft-metadata-crawler-parser && cargo run

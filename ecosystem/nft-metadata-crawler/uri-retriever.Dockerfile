FROM rust:1.70.0

ARG AUTH
ENV AUTH=$AUTH
ARG TOPIC_NAME
ENV TOPIC_NAME=$TOPIC_NAME

COPY nft-metadata-crawler-uri-retriever /nft-metadata-crawler-uri-retriever
COPY nft-metadata-crawler-utils /nft-metadata-crawler-utils

COPY nft-metadata-crawler-uri-retriever/Cargo.docker.toml /nft-metadata-crawler-uri-retriever/Cargo.toml
COPY nft-metadata-crawler-utils/Cargo.docker.toml /nft-metadata-crawler-utils/Cargo.toml

RUN cd nft-metadata-crawler-uri-retriever && cargo build --release

CMD cd nft-metadata-crawler-uri-retriever && cargo run

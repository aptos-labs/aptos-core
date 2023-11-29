### NFT Metadata Crawler Image ###

FROM indexer-builder

FROM debian-base AS nft-metadata-crawler

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \   
    apt-get update && apt-get install --no-install-recommends -y \    
    libssl1.1 \
    ca-certificates \
    net-tools \
    tcpdump \
    iproute2 \
    netcat \
    libpq-dev \
    curl

COPY --link --from=indexer-builder /aptos/dist/aptos-nft-metadata-crawler-parser /usr/local/bin/aptos-nft-metadata-crawler-parser

# The health check port
EXPOSE 8080

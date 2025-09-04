### NFT Metadata Crawler Image ###

FROM indexer-builder

FROM debian-base AS nft-metadata-crawler

COPY --link --from=indexer-builder /velor/dist/velor-nft-metadata-crawler /usr/local/bin/velor-nft-metadata-crawler

# The health check port
EXPOSE 8080

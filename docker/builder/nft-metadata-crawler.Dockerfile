### NFT Metadata Crawler Image ###

FROM indexer-builder

FROM debian-base AS nft-metadata-crawler

COPY --link --from=indexer-builder /aptos/dist/aptos-nft-metadata-crawler /usr/local/bin/aptos-nft-metadata-crawler

# The health check port
EXPOSE 8080

### NFT Metadata Crawler Image ###

FROM tools-builder

FROM debian-base AS nft-metadata-crawler

COPY --link --from=tools-builder /aptos/dist/aptos-nft-metadata-crawler /usr/local/bin/aptos-nft-metadata-crawler

# The health check port
EXPOSE 8080

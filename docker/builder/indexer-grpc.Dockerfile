### Indexer GRPC Image ###

FROM debian-base AS indexer-grpc

COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-cache-worker /usr/local/bin/aptos-indexer-grpc-cache-worker
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-file-store /usr/local/bin/aptos-indexer-grpc-file-store
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-data-service /usr/local/bin/aptos-indexer-grpc-data-service
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-file-checker /usr/local/bin/aptos-indexer-grpc-file-checker
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-data-service-v2 /usr/local/bin/aptos-indexer-grpc-data-service-v2
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-manager /usr/local/bin/aptos-indexer-grpc-manager
COPY --link --from=indexer-builder /aptos/dist/aptos-indexer-grpc-gateway /usr/local/bin/aptos-indexer-grpc-gateway

# The health check port
EXPOSE 8080
# The gRPC non-TLS port
EXPOSE 50052
# The gRPC TLS port
EXPOSE 50053

ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}

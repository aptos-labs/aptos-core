FROM aptoslabs/indexer-grpc:indexer-grpc-preview
ADD config.yaml /configs/config.yaml

WORKDIR /usr/local/bin

CMD ["aptos-indexer-grpc-parser", "-c", "/configs/config.yaml"]
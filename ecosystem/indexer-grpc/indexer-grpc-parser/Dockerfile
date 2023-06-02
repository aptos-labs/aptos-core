FROM aptoslabs/indexer-grpc:indexer-grpc-preview
ADD parser.yaml /configs/parser.yaml

WORKDIR /usr/local/bin

CMD ["aptos-indexer-grpc-parser", "-c", "/configs/parser.yaml"]
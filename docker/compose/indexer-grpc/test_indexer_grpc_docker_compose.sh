#!/bin/bash

# A quick script that checks the e2e setup for the indexer-grpc service on docker-compose

if ! command -v grpcurl &>/dev/null; then
    wget https://github.com/fullstorydev/grpcurl/releases/download/v1.8.7/grpcurl_1.8.7_linux_x86_64.tar.gz
    sha=$(shasum -a 256 grpcurl_1.8.7_linux_x86_64.tar.gz | awk '{ print $1 }')
    [ "$sha" != "b50a9c9cdbabab03c0460a7218eab4a954913d696b4d69ffb720f42d869dbdd5" ] && echo "shasum mismatch" && exit 1
    tar -xvf grpcurl_1.8.7_linux_x86_64.tar.gz
    chmod +x grpcurl
    mv grpcurl /usr/local/bin/grpcurl
    grpcurl -version
fi

# Try hitting the indexer-grpc setup in a number of ways
# 

# try getting the internal grpc on the fullnode itself
stream_time_seconds=30
start_time=$(date +%s)
timeout "${stream_time_seconds}s" grpcurl  -max-msg-sz 10000000 -d '{ "starting_version": 0 }' -import-path crates/aptos-protos/proto -proto aptos/internal/fullnode/v1/fullnode_data.proto  -plaintext 127.0.0.1:50051 aptos.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode
end_time=$(date +%s)
total_time=$((end_time - start_time))
echo "grpcurl took ${total_time} seconds to run"

if [ $total_time -lt "${stream_time_seconds}" ]; then
    echo "grpcurl exited early, which indicates failure"
    echo "RawData/GetTransactions on the aptos-node should be an endless stream"
    exit 1
fi

# try hitting the data service
stream_time_seconds=30
start_time=$(date +%s)
timeout "${stream_time_seconds}s" grpcurl  -max-msg-sz 10000000 -d '{ "starting_version": 0 }' -H "x-aptos-data-authorization:dummy_token"  -import-path crates/aptos-protos/proto -proto aptos/indexer/v1/raw_data.proto  -plaintext 127.0.0.1:50052 aptos.indexer.v1.RawData/GetTransactions
end_time=$(date +%s)
total_time=$((end_time - start_time))
echo "grpcurl took ${total_time} seconds to run"

if [ $total_time -lt "${stream_time_seconds}" ]; then
    echo "grpcurl exited early, which indicates failure"
    echo "RawData/GetTransactions on the data service should be an endless stream"
    exit 1
fi

echo "All tests passed!"

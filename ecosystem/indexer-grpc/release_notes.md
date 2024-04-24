# Release Notes

This file keeps track of the changes for indexer grpc.

## 2024.04.24

We currently offer reference implementations in Python and Typescript for processors (e.g., the NFT marketplace v2 example). Some of you might have used them and realized that there are issues, notably:
* Python implementation sometimes stalls
* Our typescript implementation is slow and has issues processing large amounts of data (e.g., gets stuck from version 300M - 400M on mainnet)
* Marketplace v2 data is very limited, so it is basically unusable.

The team’s short-term goal is to release a proper SDK, so to that end, we’ll deprecate support for the current Python / Typescript examples (as well as the marketplace v2 APIs) and focus on building the Rust, Typescript, and Python SDKs in that order of priority.

Within a couple of weeks, we’ll stop processing the following tables, and around June 1, 2024, we’ll remove them from the APIs:
* `current_nft_marketplace_auctions`
* `current_nft_marketplace_collection_offers`
* `current_nft_marketplace_listings`
* `current_nft_marketplace_token_offers`
* `nft_marketplace_activities`

From our data, we see that these APIs aren’t used often. The bigger impact may be for partners running their own Python / Typescript processors. We encourage partners to migrate to Rust, which is the only officially supported language at the moment. We’ll keep the Python and Typescript code in the repo but will not support them or update them for the moment.

## [1.0.0] - 2023.05.23

* Remove `testing` from the protobuf package path; we're going to exit alpha testing! [PR](https://github.com/aptos-labs/aptos-core/pull/8277)

* Improve the data fetching retry logic for data service. [PR](https://github.com/aptos-labs/aptos-core/pull/8169)

* Reduce the dependency tree for Indexer GRPC by 60%. [PR](https://github.com/aptos-labs/aptos-core/pull/8139)

* Introduce local file impl for `FileStoreOperator`. [PR](https://github.com/aptos-labs/aptos-core/pull/8117)

## [0.2.0] - 2023.04.25

* Split the services into internal one. [PR](https://github.com/aptos-labs/aptos-core/pull/7632)

  * Two namespaces are: `aptos.internal.fullnode.v1` and `aptos.indexer.v1`. 

  * External service is simplified since all data are sent sequentially.

* Changed the internal data format to raw bytes, this can save at 40% traffic cost. 

* Server supports request and respone compression. [PR](https://github.com/aptos-labs/aptos-core/pull/7907)


## [0.1.0] - 2023.03.28

* First version of Indexer GRPC system; alpha testing starts! 

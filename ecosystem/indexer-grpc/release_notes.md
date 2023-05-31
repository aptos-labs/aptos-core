# Release Notes

This file keeps track of the changes for indexer grpc.


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

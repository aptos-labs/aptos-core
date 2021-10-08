# Changelog

## [diem-core-v1.5.0](https://github.com/diem/diem/tree/diem-core-v1.5.0)

[Full Changelog](https://github.com/diem/diem/compare/diem-core-v1.4.1...diem-core-v1.5.0)

**Notable changes:**

**[Consensus]**

- \[2chain\] connect everything and add tests [\#9111](https://github.com/diem/diem/pull/9111)
- \[2chain\] switch to use 2-chain in Genesis module [\#9132](https://github.com/diem/diem/pull/9132)
- \[consensus\] Use PeerMetadataStorage for Consensus [\#9336](https://github.com/diem/diem/pull/9336)
- \[consensus\] make the optionally compiled code more obvious [\#9135](https://github.com/diem/diem/pull/9135)
- \[decoupled-execution\] Integration Part 1: Bridging BufferManager and EpochManager [\#9273](https://github.com/diem/diem/pull/9273)
- \[decoupled-execution\] add signing / aggregating logic [\#9148](https://github.com/diem/diem/pull/9148)
- \[decoupled-execution\] buffer manager execution phase logic [\#9126](https://github.com/diem/diem/pull/9126)
- \[decoupled-execution\] buffer manager main loop logic \(w/o message retry\) [\#9178](https://github.com/diem/diem/pull/9178)
- \[decoupled-execution\] change linkedlist to ordered dictionary [\#9316](https://github.com/diem/diem/pull/9316)
- \[decoupled-execution\] more cleanups [\#9219](https://github.com/diem/diem/pull/9219)
- \[decoupled-execution\] process\_sync\_req now tries to advance a buffer item to aggregated [\#9207](https://github.com/diem/diem/pull/9207)
- \[decoupled-execution\] retry sending commit vote [\#9196](https://github.com/diem/diem/pull/9196)
- \[decoupled-execution\] simplify code [\#9212](https://github.com/diem/diem/pull/9212)
- \[decoupled-execution\] small cleanup and implement commit proof forward & reset [\#9259](https://github.com/diem/diem/pull/9259)

**[Diem Framework]**

- \[diem-framework\] Add force shift event to CRSNs, add gate [\#9204](https://github.com/diem/diem/pull/9204)
- \[diem-framework\] Make core, experimental, and DPN Move packages [\#9246](https://github.com/diem/diem/pull/9246)
- \[diem-framework\] Port ValidatorConfig and ValidatorOperatorConfig to unit tests [\#9131](https://github.com/diem/diem/pull/9131)
- \[diem-framework\] fix DPN release tests [\#9278](https://github.com/diem/diem/pull/9278)
- \[diem-framework\] migrate authenticator tests [\#9114](https://github.com/diem/diem/pull/9114)
- \[diem-framework\] migrate registered currency tests [\#9115](https://github.com/diem/diem/pull/9115)
- \[diem-framework\] Network Identity Discovery constructs [\#8665](https://github.com/diem/diem/pull/8665)
- \[transaction-builder\] Add support for vector<vector<u8>> in script functions [\#8990](https://github.com/diem/diem/pull/8990)
- \[transaction-builder-generator\] Add support for vector<vector<u8>> [\#8997](https://github.com/diem/diem/pull/8997)

**[Executor]**

- \[executor\] parallelize execute and commit in benchmark [\#8761](https://github.com/diem/diem/pull/8761)
- \[exec\_bench\] unify diemdb\_bench and exec\_bench with same txn gen [\#9300](https://github.com/diem/diem/pull/9300)
- \[parallel\_executor\] Handle unestimated read in the executor [\#9244](https://github.com/diem/diem/pull/9244)
- Replace dyn StateView by impl StateView [\#9255](https://github.com/diem/diem/pull/9255)


**[Client]**

- json-rpc: implement missing MockDB methods to fix fuzzer [\#9128](https://github.com/diem/diem/pull/9128)
- Setup new HTTP server for the new API [\#9144](https://github.com/diem/diem/pull/9144)
- \[api\] add the api integration/smoke tests to smoke-test suite [\#9209](https://github.com/diem/diem/pull/9209)
- \[api\] better error handlings and response ledger info in header [\#9228](https://github.com/diem/diem/pull/9228)
- \[api\] first draft version api blueprint document for implemented API [\#9262](https://github.com/diem/diem/pull/9262)
- \[api\] get account modules [\#9256](https://github.com/diem/diem/pull/9256)
- \[api\] get account resources [\#9213](https://github.com/diem/diem/pull/9213)
- \[api\] get transactions [\#9267](https://github.com/diem/diem/pull/9267)
- \[api\] post /transactions to submit bcs encoded signed transaction [\#9307](https://github.com/diem/diem/pull/9307)
- \[api\] render user signed transaction payload [\#9311](https://github.com/diem/diem/pull/9311)
- \[api\] render user signed transaction signature and metadata timestamp [\#9332](https://github.com/diem/diem/pull/9332)

**[Mempool]**

- \[mempool/state-sync\] Make PeerNetworkId copyable [\#9225](https://github.com/diem/diem/pull/9225)
- \[mempool/state-sync\] Remove NodeNetworkId & modernize PeerNetworkId [\#9191](https://github.com/diem/diem/pull/9191)
- \[mempool\] Finish NetworkInterface integration with mempool [\#9333](https://github.com/diem/diem/pull/9333)
- \[mempool\] improve mempool and vm validator mocks [\#9276](https://github.com/diem/diem/pull/9276)


**[Move]**

- Updated to version 1.5 of Move: see the [Move release note](https://github.com/diem/diem/blob/main/language/RELEASES.md) for details.

**[Network]**

- Refactor TransportManager to better indicate direction & have more comments / code reuse [\#9120](https://github.com/diem/diem/pull/9120)
- \[network\] Fix JSON consensus protocol name [\#9133](https://github.com/diem/diem/pull/9133)
- \[network\] Integrate NetworkInterface paradigm with Healthchecker [\#9031](https://github.com/diem/diem/pull/9031)
- \[network\] Make NetworkId & NetworkContext copyable [\#9203](https://github.com/diem/diem/pull/9203)
- \[network\] Make NetworkInterface use PeerNetworkId as a key [\#9245](https://github.com/diem/diem/pull/9245)
- \[network\] clean up SupportedProtocols. support iterating with unknown ProtocolIds [\#9266](https://github.com/diem/diem/pull/9266)
- \[network\] fix small bug in handshake negotiation [\#9258](https://github.com/diem/diem/pull/9258)
- \[network\] support registering client-only or service-only applications [\#9297](https://github.com/diem/diem/pull/9297)

**[Operational Tool]**

- \[reconfiguration\] another attempt to fix the bug and better test cove… [\#9119](https://github.com/diem/diem/pull/9119)
- \[docker\] add experimental move bytecode [\#9234](https://github.com/diem/diem/pull/9234)
- logger: allow configuring what logs get sent to the remote logging service [\#9181](https://github.com/diem/diem/pull/9181)
- add runner for starting a Diem node with custom genesis modules [\#9026](https://github.com/diem/diem/pull/9026)
- \[diem node\] add option to specify genesis modules for testing [\#9198](https://github.com/diem/diem/pull/9198)

**[State Sync]**

- \[state\_sync\_v2\]\[diemdb\] store write\_set to diemdb for state\_sync v2 [\#9326](https://github.com/diem/diem/pull/9326)
- \[State Sync\] Add a new Data Streaming Service crate and a client implementation. [\#9325](https://github.com/diem/diem/pull/9325)
- \[State Sync\] Add a new EventSubscriptionService and tests [\#9117](https://github.com/diem/diem/pull/9117)
- \[State Sync\] Add a new Storage Service implementation \(server-side\) [\#9199](https://github.com/diem/diem/pull/9199)
- \[State Sync\] Add the \(skeleton\) Diem Data Client API and crate. [\#9279](https://github.com/diem/diem/pull/9279)
- \[State Sync\] Added "include\_events" flag to transaction proof requests. [\#9314](https://github.com/diem/diem/pull/9314)
- \[State Sync\] Avoid using is\_ok\(\) in tests. [\#9124](https://github.com/diem/diem/pull/9124)
- \[State Sync\] Complete the storage service skeleton \(server-side\). [\#9232](https://github.com/diem/diem/pull/9232)
- \[State Sync\] Don't panic on failures to read on-chain configs. [\#9130](https://github.com/diem/diem/pull/9130)
- \[State Sync\] Move verify\(\) logic into `SparseMerkleRangeProof`. [\#9253](https://github.com/diem/diem/pull/9253)
- \[State Sync\] Plug in the new EventSubscriptionService. [\#9164](https://github.com/diem/diem/pull/9164)
- \[State Sync\] Return the ledger infos in the DataSummary and fetch the number of accounts [\#9277](https://github.com/diem/diem/pull/9277)
- \[State Sync\] Small cleanups to notification listener tests. [\#9122](https://github.com/diem/diem/pull/9122)
- \[State Sync\] Update the EventSubscriptionService to handle missing on-chain configs. [\#9152](https://github.com/diem/diem/pull/9152)
- \[State Sync\] Use the claim crate for better test assertions [\#9163](https://github.com/diem/diem/pull/9163)

**[Storage]**

- \[benchmark\] refactor exec/diemdb benchmarks to store real txn data in db [\#9247](https://github.com/diem/diem/pull/9247)
- \[schemaDB\] add checkpoint support [\#9221](https://github.com/diem/diem/pull/9221)
- \[diemdb\] add API get\_transaction\_by\_hash [\#9206](https://github.com/diem/diem/pull/9206)
- \[diemdb\] add transaction\_by\_hash cf [\#9171](https://github.com/diem/diem/pull/9171)
- \[storage\] get\_account\_count\(\) [\#9304](https://github.com/diem/diem/pull/9304)
- \[storage\] serialize StorageConfig::account\_count\_migration [\#9350](https://github.com/diem/diem/pull/9350)

**[SDK/Ecosystem]**

- sdk: release v0.0.3 [\#9321](https://github.com/diem/diem/pull/9321)
- \[shuffle\] Add Ability to Publish User Modules [\#9161](https://github.com/diem/diem/pull/9161)
- \[shuffle\] Add Message module example [\#9265](https://github.com/diem/diem/pull/9265)
- \[shuffle\] Introduce shuffle CLI cmds: new,node [\#9227](https://github.com/diem/diem/pull/9227)
- \[shuffle\] change node cmd to load test env [\#9347](https://github.com/diem/diem/pull/9347)
- \[shuffle\] remove copy of DF modules from shuffle [\#9170](https://github.com/diem/diem/pull/9170)
- \[shuffle\] set versions for all deno.land imports [\#9345](https://github.com/diem/diem/pull/9345)
- \[shuffle\] use move package system instead of manual files [\#9248](https://github.com/diem/diem/pull/9248)
- Shuffle end to end v0 [\#9272](https://github.com/diem/diem/pull/9272)

**[TCB]**

- \[safety-rules\] decouple waypoint update and epoch changes [\#9102](https://github.com/diem/diem/pull/9102)

\* *This Changelog was partially generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*

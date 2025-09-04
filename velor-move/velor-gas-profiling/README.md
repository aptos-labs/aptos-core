# Gas Profiling

## Overview
This crate implements a gas profiler that can be plugged into the Velor VM to generate comprehensive traces of gas usage, referred to as the transaction gas log.
It also contains a module for visualizing the transaction gas log, in the form of a flamegraph.

## Running the Gas Profiler
You can run the gas profiler by appending the `--profile-gas` option to the velor cli's `move publish`, `move run` & `move run-script` commands. Here is an example:
```
>> cargo run -p velor -- move publish --profile-gas
    Finished dev [unoptimized + debuginfo] target(s) in 0.51s
     Running `/home/vgao/velor-core/target/debug/velor move publish --profile-gas`
Compiling, may take a little while to download git dependencies...
BUILDING empty_fun
package size 427 bytes

Simulating transaction locally with the gas profiler...
This is still experimental so results may be inaccurate.

Execution & IO Gas flamegraph saved to gas-profiling/txn-69e19ee4-0x1-code-publish_package_txn.exec_io.svg
Storage fee flamegraph saved to gas-profiling/txn-69e19ee4-0x1-code-publish_package_txn.storage.svg

{
  "Result": {
    "transaction_hash": "0x69e19ee4cc89cb1f84ee21a46e6b281bd8696115aa332275eca38c4857818dfe",
    "gas_used": 1007,
    "gas_unit_price": 100,
    "sender": "dbcbe741d003a7369d87ec8717afb5df425977106497052f96f4e236372f7dd5",
    "success": true,
    "version": 473269362,
    "vm_status": "status EXECUTED of type Execution"
  }
}
```

## Performance Implications
It is important to note that the current gas profiler implementation is quite heavy-weight since it records every Move bytecode instruction and its cost. If real-time gas profiling is required, it is recommended to develop a custom profiler that operates on aggregated data. A standard light-weight implementation may be provided in the future.

## Known Issues & Future Plans
1. While addresses are truncated in the flamegraphs, they are still somewhat cumbersome. We plan to come up with a smart rendering algorithm that omits addresses, provided that the functions/items can still be unambiguously identified.
2. At present, the storage fee graph does not display the free quota for events. We plan to address this in a future update.

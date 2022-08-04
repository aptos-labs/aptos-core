---
id: forge cli
title: Forge CLI
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/testsuite/forge-cli/README.md
---

# Forge CLI

This crate contains the Forge command line interface (CLI) tool. This enables users to
run local and remote forge swarms (i.e., networks of validator nodes). For example, to
deploy a local validator swarm, run:

```
cargo run -p forge-cli -- --suite "run_forever" --num-validators 3 test local-swarm
```

This will start a local swarm of 3 validators, each running in their own process. The
swarm will run forever, unless manually killed. The output will display the locations
of the swarm files (e.g., the genesis files, logs, node configurations, etc.) and the
commands that were run to start each node. The process id (PID) of each node is also
displayed when it starts.

Using the information from the above command, you could stop a single node and restart
it, e.g., run:

```
kill -9 <Node PID>
cargo run -p aptos-node -- -f <Location to the node configuration file outputted above>
```

To see all tool usage options, run:
```
cargo run -p forge-cli --help
```

// TODO: add more detailed usage information. There's a lot more that users can do!

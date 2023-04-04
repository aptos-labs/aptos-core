---
title: "Telemetry"
slug: "telemetry"
---

When you operate a node on an Aptos network, your node can be set to send telemetry data to Aptos Labs. You can disable telemetry at any point. If telemetry remains enabled, Aptos node binary will send telemetry data in the background.

The Aptos node binary running on your node collects telemetry data such as software version, operating system information and the IP address of your node. This telemetry data is used to enhance the decentralization of the network.

:::tip No personal information is collected
The Aptos node binary does **not** collect personal information such as usernames or email addresses.
:::

## Metrics collected

### Core metrics

- Core metrics: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/core_metrics.rs#L14-L29](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/core_metrics.rs#L14-L29).

### Node information

The public IP address of the node and core metrics, including node type, synced version and number of network connections.

- **Node configuration as a mapping of string key to JSON map**: [https://github.com/aptos-labs/aptos-core/blob/main/config/src/config/mod.rs#L63-L97](https://github.com/aptos-labs/aptos-core/blob/main/config/src/config/mod.rs#L63-L97).

### CLI telemetry

The commands and subcommands run by the Aptos CLI tool.

- **CLI metrics**: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/cli_metrics.rs#L12-L15](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/cli_metrics.rs#L12-L15).
- **Build information**: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-build-info/src/lib.rs#L8-L20](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-build-info/src/lib.rs#L8-L20).

### Network metrics

- **Network metrics**: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/network_metrics.rs#L12-L17](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/network_metrics.rs#L12-L17).

### Build information

Rust build information including the versions of Rust, cargo, build target architecture and the build tag.

- **Build information**: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-build-info/src/lib.rs#L8-L20](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-build-info/src/lib.rs#L8-L20)

### System information

System information including operating system information (including versions), hardware information and resource utilization (including CPU, memory and disk).

- **System information**: [https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/system_information.rs#L14-L32](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-telemetry/src/system_information.rs#L14-L32).

### Others

- **Metrics**: All the [Prometheus](https://prometheus.io/) metrics that are collected within the node.
- **Logs**: Logs of warn-level and higher level, with the ability to collect up to debug logs.

## Disabling telemetry

On macOS and Linux, you can set the following environment variables to control the metrics sent by your node. For example, to disable all telemetry, set the  `APTOS_DISABLE_TELEMETRY` environment variable to `true` as shown below:

```bash
export APTOS_DISABLE_TELEMETRY=true
```

The above example only disables telemetry for a single session in the current terminal where you ran the above command. To disable it permanently on your node, include it in your startup profile, as below: 

```bash
echo "export APTOS_DISABLE_TELEMETRY=true" >> ~/.profile
source ~/.profile
```

:::tip All telemetry is ON by default.
All the below variables are set by default to `false`, i.e., sending of these telemetry metrics is enabled. Set them to `true` to disable telemetry.
:::

- `APTOS_DISABLE_TELEMETRY`: This disables all telemetry emission from the node including sending to the GA4 service.
- `APTOS_FORCE_ENABLE_TELEMETRY`: This overrides the chain ID check and forces the node to send telemetry regardless of whether remote service accepts or not.
- `APTOS_DISABLE_TELEMETRY_PUSH_METRICS`: This disables sending the [Prometheus](https://prometheus.io/) metrics.
- `APTOS_DISABLE_TELEMETRY_PUSH_LOGS`: This disables sending the logs.
- `APTOS_DISBALE_TELEMETRY_PUSH_EVENTS`: This disables sending the custom events.
- `APTOS_DISABLE_LOG_ENV_POLLING`: This disables the dynamic ability to send verbose logs.
- `APTOS_DISABLE_PROMETHEUS_NODE_METRICS`: This disables sending the node resource metrics such as system CPU, memory, etc.

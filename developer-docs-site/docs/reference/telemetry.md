---
title: "Telemetry"
slug: "telemetry"
---

At Aptos Labs, we develop software and services for the greater Aptos community and ecosystem. On top of community feedback, we use telemetry to help improve the decentralization of the network by understanding how our software is being deployed and run.

The Aptos node binary collects telemetry such as software version, operating system information, and IP address. See [Types of information collected](#types-of-information-collected).

The Aptos node binary does **not** collect personal information such as usernames or email addresses.

Users can disable telemetry at any point. If telemetry remains enabled, Aptos node binary will send telemetry data in the background.

# Disabling telemetry

On macOs and Linux, you can disable telemetry by setting the `APTOS_TELEMETRY_DISABLE` environment variable to any value.

```
export APTOS_TELEMETRY_DISABLE=true
```

The above example only disables telemetry for a single session or terminal. To disable it everywhere, you must do so at shell startup.

```
echo "export APTOS_TELEMETRY_DISABLE=true" >> ~/.profile
source ~/.profile
```

# Types of information collected

* **Usage information** - Commands and subcommands that are run
* **System information** - Operating system (Windows, Linux, macOS) and kernel information, CPU and memory utilization
* **Software information** - Version of the Aptos node binary
* **Node information** - Public IP address, number of inbound and outbound Aptos node connections

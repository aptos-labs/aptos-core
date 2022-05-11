---
title: "Troubleshooting FullNode Setup"
slug: "troubleshooting-fullnode-setup"
sidebar_position: 13
---

# Troubleshooting FullNode Setup

**Q: When starting the node, it throws a YAML-parsing error.**

**A:** YAML files are sensitive to formatting errors. Use a dedicated YAML editor or use a YAML syntax checker in your preferred editor to check if each line in the YAML file is indented correctly. See the example YAML file in: `docker/compose/public_full_node/public_full_node.yaml`.

**Q: When I start a node with `cargo run -p ...` command I get "Unable to fetch any peers to poll" error.**

It looks like I have no peers on the available node testers. I have no output when I run:

```
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{type=\"synced\"}"
```

Also my sync version does not increase, indicating that I am not syncing.

**A:** The Devnet validator FullNodes will only accept a maximum of connections. If Aptos devnet is experiencing high network connection volume, your FullNode might not able to connect. It is also possible that you do not have proper network configuration with firewall rules to allow outbound traffic.

You can workaround this by:

1. Checking your network configuration, and
2. Adding a seed peer to connect to, in your `public_full_node.yaml` file. See this section: [Add upstream seed peers](run-a-fullnode#add-upstream-seed-peers).

For example, after you add a single peer to the `seeds` section in your `public_full_node.yaml` file like below, restart the `cargo run -p ...` command:

```YAML
full_node_networks:
    - discovery_method: "onchain"
      # The network must have a listen address to specify protocols. This runs it locally to
      # prevent remote, incoming connections.
      listen_address: "/ip4/127.0.0.1/tcp/6180"
      network_id: "public"
      # Define the upstream peers to connect to
      seeds:
        bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a:
            addresses:
            - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/ln-handshake/0"
            role: "Upstream"

```


[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev

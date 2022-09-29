---
title: "Issues and Workarounds"
slug: "issues-and-workarounds"
---

# Issues and Workarounds

This page documents issues that frequently come up and the suggested workarounds. 

:::tip Help keep this page up-to-date
If you found an issue that is not on this page, submit a [GitHub Issue](https://github.com/aptos-labs/aptos-core/issues). Make sure to follow the issue format used in this document. 
:::

## Nodes

### Terraform "Connection Refused" error

#### Description

When running terraform, the command errors out with a connection refused error message.

  ```
  Error: Get "http://localhost/api/v1/namespaces/aptos": dial tcp 127.0.0.1:80: connect: connection refused
  ```

#### Workaround

This likely means that the state of the install is out of sync with the saved terraform state file located in the storage bucket (configured during `terraform init` statement).  This could happen if the cluster or other components were deleted outside of terraform, or if terraform had an error and did not finish.  Use the following commands to check the state.  Delete the state that is related to the error message.  You will likely need to run terraform destroy, clean up the environment, and run the terraform script again.  

  ```
  terraform state list

  terraform state rm <state>
  ```

### Fullnode "NoAvailablePeers" error

#### Description

If your node cannot state sync, and the logs are showing "NoAvailablePeers", it's likely due to network congestion. 

#### Workaround

You can try add some extra upstream peers for your fullnode to state sync from. See the guide [Add upstream seed peers](nodes/full-node/fullnode-source-code-or-docker.md#add-upstream-seed-peers).

### Starting a node throws a YAML-parsing error

#### Workaround

YAML files are sensitive to formatting errors. Use a dedicated YAML editor or use a YAML syntax checker in your preferred editor to check if each line in the YAML file is indented correctly.

### "Unable to fetch any peers to poll" error

#### Description

When starting a node with the `cargo run -p ...` command, you receive a "Unable to fetch any peers to poll" error. It looks like you have no peers on the available node testers. You have no output when running:

```bash
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{type=\"synced\"}"
```

Also sync version does not increase, indicating syncing is not working.

#### Workaround

The devnet validator fullnodes will accept only a maximum of connections. If Aptos devnet is experiencing high network connection volume, your fullnode might not able to connect. It is also possible that you do not have proper network configuration with firewall rules to allow outbound traffic.

You can workaround this by:

1. Checking your network configuration.
2. Adding a seed peer to connect to, in your `public_full_node.yaml` file. See [Add upstream seed peers](nodes/full-node/fullnode-source-code-or-docker.md#add-upstream-seed-peers).

For example, after you add a single peer to the `seeds` section in your `public_full_node.yaml` file like below, restart the `cargo run -p ...` command:

```yaml
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
            - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/handshake/0"
            role: "Upstream"
```

### Node liveness issues

#### Workaround

If your validator node is facing persistent issues, for example, it is unable to propose or fails to synchronize, open an [aptos-ait2](https://github.com/aptos-labs/aptos-ait2/issues) GitHub issue and provide the following:
- Your node setup, i.e., if you're running it from source, Docker or Terraform. Include the source code version, i.e., the image tag or branch).
- A description of the issues you are facing and how long they have been occurring.
- **Important**: The logs for your node (going as far back as possible). Without the detailed logs, the Aptos team will likely be unable to debug the issue.
- We may also ask you to enable the debug logs for the node. You can do this by updating your node configuration file (e.g., `validator.yaml`) by adding:
```yaml
 logger:
   level: DEBUG
```
- Make sure to also include any other information you think might be useful and whether or not restarting your validator helps.





[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev




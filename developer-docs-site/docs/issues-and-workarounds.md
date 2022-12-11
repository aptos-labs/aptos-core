---
title: "Issues and Workarounds"
slug: "issues-and-workarounds"
---

# Issues and Workarounds

This page provides workarounds and answers for issues and questions that frequently come up. 

:::tip Help keep this page up-to-date
If you found an issue that is not on this page, submit a [GitHub Issue](https://github.com/aptos-labs/aptos-core/issues). Make sure to follow the issue format used in this document. 
:::

## Nodes

### Logs slowing down system and hindering performance

#### Description

Encountered problems with very large logs that were written to the system. They accumulate quickly and could easily be more than 1G in a couple of days, greatly slowing down the system and the performance of the disk.

#### Workaround

Run `aptos-node` as a system service and forward logs to syslog. See the following issue for a detailed description of the problem and suggested solution:
https://github.com/aptos-labs/aptos-core/issues/5522

### Invalid EpochChangeProof: Waypoint value mismatch

#### Description

Receive this error from a validator node:

```json
{"error":"Invalid EpochChangeProof: Waypoint value mismatch: waypoint value = 3384a932349524093cda8cea714691e668d668fb34260d8a5f77c667d7724372, given value = 81ee9bd880acd25ad617e55913b7345dc01b861adf43971259a22e9a5c82315c","event":"error","name":"initialize"}
```

#### Workaround

Delete the `secure-data.json` file because very likely you are using an older version of it.  Check your `validator.yaml` file and you will see something like `path: /opt/aptos/data/secure-data.json`. Or see [Bootstrapping validator node](nodes/validator-node/operator/connect-to-aptos-network.md#bootstrapping-validator-node) for the location of this file. For Docker, you can delete this file by `docker` commands such as: `docker-compose down --volumes` (check the `docker-compose` help). Finally, **remember to restart the node. **

### How to find out when the next epoch starts

:::tip Current epoch duration
The Aptos current epoch duration is 1 hour.
:::

You can find out when the next epoch starts in multiple ways: 

**Use the CLI**
```bash
aptos node show-epoch-info --url https://fullnode.mainnet.aptoslabs.com/v1
```
which produces an output like below (example output for mainnet):
```json
{
  "Result": {
    "epoch": 692,
    "epoch_interval_secs": 3600,
    "current_epoch_start_time": {
      "unix_time": 1665429258117522,
      "utc_time": "2022-10-10T19:14:18.117522Z"
    },
    "next_epoch_start_time": {
      "unix_time": 1665436458117522,
      "utc_time": "2022-10-10T21:14:18.117522Z"
    }
  }
}
```

**Find it in your stake pool information output**

You can see when the next epoch starts in the output of the command `aptos node get-stake-pool`. See [Checking your stake pool information](/nodes/validator-node/operator/staking-pool-operations/#checking-your-stake-pool-information).

Finally, you can use Aptos Explorer and an online epoch converter to find out when the next epoch starts. See below:

**You can use the Aptos Explorer and epoch converter**

1. Go to account `0x1` page on the Aptos Explorer by [clicking here](https://explorer.aptoslabs.com/account/0x1). Make sure the correct network (mainnet or testnet or devnet) is selected at the top right.
2. Switch to **RESOURCES** tab.
3. Using the browser search (Ctrl-f, do not use the **Search transactions** field), search for `last_reconfiguration_time`. You will find the last epoch transition timestamp in microseconds. The text display looks like this:
  ```json
  {
    "epoch": "25",
    "events": {
      "counter": "25",
      "guid": {
        "id": {
          "addr": "0x1",
          "creation_num": "2"
        }
      }
    },
    "last_reconfiguration_time": "1664919592960637"
  }
  ```

4. Go to https://www.epochconverter.com/ and include the epoch timestamp to convert it to a human-readable date. 

### How to check if a validator address is in the validator set

You can check if a validator address is in the Aptos validator set either on the command line or by using the Aptos Explorer.

**CLI** 

Run the below command:
```bash
aptos node show-validator-set --profile operator | jq -r '.Result.active_validators[].addr' | grep <stake pool address>
```

And ensure you see the validator in the output.

**Aptos Explorer**

Follow these steps on the Aptos Explorer:

1. Go to account [`0x1`](https://explorer.aptoslabs.com/account/0x1) page on the Aptos Explorer.
1. Make sure the correct network (mainnet or testnet or devnet) is selected at the top right.
2. Switch to the **RESOURCES** tab.
3. Using the browser search (Ctrl-f, do not use the **Search transactions** field), search for the validator address. 

### How to find stake pool address

To find out which stake pool address to use (for example, to bootstrap your node), run the below command. This example is for mainnet. See the `--url` value for testnet or devnet in [Aptos Blockchain Deployments](/docs/nodes/aptos-deployments.md). Also see [Bootstrapping validator node](nodes/validator-node/operator/connect-to-aptos-network.md#bootstrapping-validator-node):

```bash
aptos node get-stake-pool \
  --owner-address 0x0756c80f0597fc221fe043d5388949b34151a4efe5753965bbfb0ed7d0be08ea \
  --url https://fullnode.mainnet.aptoslabs.com/v1
```

### How to check if an address is the correct stake pool address or a correct validator address

Follow these steps on the Aptos Explorer:

1. Go to account [`0x1`](https://explorer.aptoslabs.com/account/0x1) page on the Aptos Explorer.
1.  Make sure the correct network (mainnet or testnet or devnet) is selected at the top right.
2. Switch to **RESOURCES** tab.
3. Using the browser search (Ctrl-f, do not use the **Search transactions** field), search for `StakePool`. The address with the `StakePool` resource is the correct stake pool address.
4. You can double-check by searching for the operator and seeing if that’s your operator address. 

### How to see previous epoch rewards

To see the previous epoch rewards for a given pool address, click on a URL of the below format. This example is for mainnet and for the pool address `0x2b32ede8ef4805487eff7b283571789e0f4d10766d5cb5691fe880b76f21e7e4`. Use the network and pool address of your choice in this place:

```html
https://fullnode.mainnet.aptoslabs.com/v1/accounts/0x2b32ede8ef4805487eff7b283571789e0f4d10766d5cb5691fe880b76f21e7e4/events/10
```

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

### How to find chain ID of my node

On your node, run this command to find out the chain ID of your node:

```bash
curl http://127.0.0.1:8080/v1
```

### Fullnode "NoAvailablePeers" error

#### Description

If your node cannot state sync, and the logs are showing "NoAvailablePeers", it's likely due to network congestion. 

#### Workaround

You can try add some extra upstream peers for your fullnode to state sync from. See the section [Connecting your fullnode to seed peers](/nodes/full-node/fullnode-network-connections#connecting-your-fullnode-to-seed-peers).

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
2. Adding a seed peer to connect to, in your `public_full_node.yaml` file. See [Connecting your fullnode to seed peers](/nodes/full-node/fullnode-network-connections#connecting-your-fullnode-to-seed-peers).

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




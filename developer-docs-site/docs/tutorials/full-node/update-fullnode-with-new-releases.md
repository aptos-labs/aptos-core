---
title: "Update FullNode With New Releases"
slug: "update-fullnode-with-new-releases"
sidebar_position: 11
---

# Update FullNode With New Releases

:::info ✨ Devnet New Address Format ✨
Aptos addresses are now 32-bytes instead of 16-bytes. If you added seed peers before, make sure you update them to the new 32-bytes format.

If you are using static identity, you can regenerate your identity following the instructions in the section [Creating a static identity for a FullNode](network-identity-fullnode#creating-a-static-identity-for-a-fullnode).
:::info


When `devnet` is wiped and updated with newer versions, you will need to update your FullNode as well. If you do not, your FullNode will not continue to synchronize with the network. To update your FullNode, follow these steps:

1. Shutdown your FullNode.

2. Delete the data folder (the directory path is what you specified in the configuration file, e.g., `public_full_node.yaml`).

    - The default data folder is `/opt/aptos/data` if you run the binary, and `DIRECTORY_WITH_YOUR_DOCKER_COMPOSE_db` if you run the Docker.
    - Use `docker volume rm DIRECTORY_WITH_YOUR_DOCKER_COMPOSE_db -f` and replace `DIRECTORY_WITH_YOUR_DOCKER_COMPOSE` with the directory name from which you started the Docker.

3. Delete the `genesis.blob` file and `waypoint.txt` file (depending on how you configured it, you might not have this file and may instead have a `waypoint` directly in your configuration file).

4. If you use the Rust binary, pull the latest of `devnet` branch, and build the binary again.

5. Download the latest Docker images with: `docker pull docker.io/aptoslab/validator:devnet`.

5. Download the new [genesis.blob][devnet_genesis] file and the new [waypoint][devnet_waypoint].

6. Update the configuration file (e.g., `public_full_node.yaml`) with the new waypoint (if you configure the waypoint directly there).

7. Restart the FullNode.

8. See the [Verify initial synchronization](run-a-fullnode#verify-initial-synchronization) section for checking if the FullNode is syncing again.


[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev

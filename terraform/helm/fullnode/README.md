Aptos Public Fullnode Deployment
================================

This Helm chart deploys a public fullnode for the Aptos blockchain network. The
fullnode connects to Aptos validators and synchronises the blockchain state to
a persistent volume. It provides a [REST API][] for interacting with
the blockchain.


Configuration
-------------

See [values.yaml][] for the full list of options you can configure.

* `chain.name`: Select which blockchain to connect to. Current values:
  - "testnet": Aptos testnet
* `storage.class`: This needs to be set to a StorageClass available in your
  Kubernetes cluster. Example values:
  - AWS: "gp2"
  - GCP: "standard"
  - Azure: "managed"
* `service.type`: By default the REST API endpoint is only exposed within the
  Kubernetes cluster. If you want to expose it externally set this to
  "LoadBalancer".
* `service.loadBalancerSourceRanges`: If you enable the LoadBalancer service you
  can set this to a list of IP ranges to restrict access to.
* `image.tag`: Select the image tag to deploy. Backup and restore images are specified separately in `backup.image.tag` and `restore.image.tag`. For a full list of image tags, check out the [Aptos dockerhub][]. Some useful tags:
  - `devnet`: the image tag devnet currently running

Connecting to Testnet
-------------

To connect to the Aptos devnet, you must have the correct genesis blob and waypoint. The source of truth for these are hosted here:
* https://devnet.aptoslabs.com/waypoint.txt
* https://devnet.aptoslabs.com/genesis.blob

The waypoint is configured as a helm value in `aptos_chains.devnet.waypoint`, and the genesis blob should be copied to `files/genesis/testnet.blob`

You may also need to change the chain era helm value in `chain.era` to the source of truth hosted at:
* https://devnet.aptoslabs.com/era.txt

Deployment
----------

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Configure `kubectl` with the Kubernetes cluster you wish to use.
3. Install the release, setting any options:

       $ helm install fullnode --set storage.class=gp2 .


[REST API]: https://github.com/aptos-labs/aptos-core/blob/main/api/doc/v0/openapi.yaml
[values.yaml]: values.yaml
[Aptos dockerhub]: https://hub.docker.com/r/aptoslabs/validator/tags?page=1&ordering=last_updated

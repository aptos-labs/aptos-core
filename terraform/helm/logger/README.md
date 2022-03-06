Aptos Logger Deployment
================================

This Helm chart deploys a central logger that aggregates logs from aptos nodes
using [Vector][]. The logger can be used to output logs to our central logging
system using mutual TLS, to file for debugging purposes, and any other outputs
possible with Vector output configuration.

Note to partners: please don't point this logger towards our premainnet or mainnet
central logging stack. We'd like to keep that for validators and key Association-run
public fullnodes.

Configuration
-------------

See [values.yaml][] for the full list of options you can configure.

* `logging.vector.logToFile`: logs to /tmp/logs for debugging purposes
* `logging.vector.outputs`: your own custom vector outputs
* `loggingClientCert`, `loggingClientKey`, `loggingCA`, `loggingCentralHost`: for mutual TLS with a central loging system

There exist template helm values files in the `values` directory, for premainnet and mainnet.

Deployment
----------

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Configure `kubectl` with the Kubernetes cluster you wish to use.
3. Set the value `logger.name` to `<owner-name>-<node-type>`, e.g. `novi-pfn`
4. Set the value `serviceAccount.name` to an existing fullnode or validator service account, or do a role binding, e.g. with `aptos-validator-psp`.
5. Configure any of the other helm values if applicable. An example to connect to `mainnet` is included in the `values` directory. If unset, the fullnode will connect to premainnet by default.
6. Install the release, setting any options:

       $ helm install fullnode-logger --set logging.vector.logToFile=true .

[Vector]: https://vector.dev/
[values.yaml]: values.yaml

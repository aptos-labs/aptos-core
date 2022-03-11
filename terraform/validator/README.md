Aptos Validator Deployment
=========================

This directory provides terraforms to deploy a single Aptos Validator node. The cloud-specific Terraform configs will
create a Kubernetes cluster from scratch and then install the Kubernetes configs. They also deploy Hashicorp
Vault outside of Kubernetes for storing validator keys and sensitive data.

* If you want to deploy a testing network, take a look of the [Aptos Testnet Terraform][] or [Aptos Testnet docker compose][]
* If you want to deploy a fullnode, take a look of the [Aptos fullnode helm][] or [Aptos fullnode docker compose][]


[Aptos Testnet Terraform]: ../testnet
[Aptos Testnet docker compose]: ../../docker/compose/validator-testnet
[Aptos fullnode helm]: ../helm/fullnode
[Aptos fullnode docker compose]: ../../docker/compose/public_full_node
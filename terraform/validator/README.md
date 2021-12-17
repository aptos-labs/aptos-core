Diem Validator Deployment
=========================

This directory provides terraforms to deploy a Diem Validator node. The cloud-specific Terraform configs will
create a Kubernetes cluster from scratch and then install the Kubernetes configs. They also deploy Hashicorp
Vault outside of Kubernetes for storing validator keys and sensitive data.

* If you are deploying a brand new validator please see [INSTALL.md][].


[INSTALL.md]: INSTALL.md

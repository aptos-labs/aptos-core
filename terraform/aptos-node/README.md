Aptos Node Deployment
=========================

This directory provides terraforms to deploy a Aptos node, which includes a validator node and a fullnode, it also comes with a HAProxy in the helm chart so that it's easy to manage incoming traffic. The cloud-specific Terraform configs will create a Kubernetes cluster and then install the helm charts.
Diem Validator Vault Initialisation
===================================

This directory contains Terraform configuration to initialise the keys and data
in Hashicorp Vault needed by a Diem Validator deployment. It does not deploy
Vault itself; for that please see the cloud-specific Terraform configs. You
will need a root token (or similar) to apply this configuration.

What it does
------------

You should review the Terraform configuration to understand what is being done
in your Vault deployment, but at a high level it creates:

* KV-v2 data: `diem/owner_account`, `diem/operator_account`, `diem/waypoint`,
  `diem/safety_data`
* Transit keys: `diem__owner`, `diem__operator`, `diem__consensus`,
  `diem__validator_network`, `diem__fullnode_network`, `diem__execution`
* Policies: `diem-validator`, `diem-safety-rules`, `diem-key-manager`,
  `diem-fullnode`, `diem-management`

Kubernetes Integration
----------------------

This also configures authentication with the Kubernetes cluster which the Diem
Validator runs in, and maps the Kubernetes Service Accounts to the appropriate
Vault policies. If you want to configure authentication yourself please delete
`kubernetes.tf` before applying. Otherwise you will need to provide some
information about your Kubernetes cluster. If you are using the Diem
cloud-specific Terraform configs to create your Kubernetes cluster, this
information will be written to `kubernetes.json` by that Terraform and can be
directly provided to this Terraform.


Setting up Hashicorp Vault Locally
----------------------------------
If you don't use Terraform or Cloud infrastructure, you can follow this instructions to setup Vault locally.

1. Install Vault and set up a vault server instance https://learn.hashicorp.com/tutorials/vault/getting-started-install?in=vault/getting-started

2. Initialize Vault server https://learn.hashicorp.com/tutorials/vault/getting-started-deploy?in=vault/getting-started. Record the Recovery Key and Initial Root Token securely (e.g. in a password manager)

3. Create vault policies used by validator deployment
    * Create the policy content in HCL files (json format compatible) https://learn.hashicorp.com/tutorials/vault/getting-started-policies?in=vault/getting-started#policy-format
    * Write the policies into vault server https://learn.hashicorp.com/tutorials/vault/getting-started-policies?in=vault/getting-started#write-a-policy
    * List of policies: diem-validator, diem-safety-rules, diem-key-manager, diem-fullnode, diem-management. Details of each policy can be found in this file [policy.tf][]

4. Create KV-v2 data used by validator deployment
    * List of KV-v2 data can be found in this file as “vault_generic_secret” [main.tf][]
    * Writing each of the KV-v2 secrets into secret engine https://learn.hashicorp.com/tutorials/vault/getting-started-first-secret?in=vault/getting-started#writing-a-secret

5. Create transit keys used by validator deployment
    * Enable transit engine https://learn.hashicorp.com/tutorials/vault/eaas-transit#configure-transit-secrets-engine
    * Create transit keys in ED25519 type https://www.vaultproject.io/api/secret/transit#create-key
    * List of transit keys can be found in this file as “vault_transit_secret_backend_key” [main.tf][]

[policy.tf]: policy.tf
[main.tf]: main.tf

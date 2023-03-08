---
title: "Identity and Configuration"
slug: "identity-and-configuration"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';


# Node Identity and Configuration

When installing a node on an Aptos network, the installation steps require you to work with identities and configurations. This document describes how to interpret the terms **identity** and **configuration**, and presents a description of the identity YAML files.

## Concept

This section presents a mental-model view of an identity and configuration. It is meant to help make the node installation process easy.

The terms **identity** and **configuration** should be understood in the following way:

- The terms **validator node**, **fullnode**, and **validator fullnode** refer to the machine (physical or virtual).
- The terms **operator**, **owner** and **voter** refer to the persona. 
- A machine has both an identity and a configuration. They are defined in separate YAML files. A persona's identity and configuration are combined into a single YAML file.

### Machine

#### Identity

Machine **identity** is defined in a YAML file. An identity is established by means of keys. A machine identity YAML contains only private keys. Moreover, an identity YAML always contains the associated blockchain account address.

A machine identity YAML has the string `identity` in its name. For example:

- validator-**identity**.yaml contains the private keys for the validator node. 
- validator-full-node-**identity**.yaml contains the private keys for validator fullnode and public fullnode. 

Hence if you are looking for your machine’s private keys, look for YAML filenames with  `identity` in them.

#### Configuration

Machine **configuration** is also defined in a YAML file. A machine configuration YAML **never contains any key, public or private**. For example, the configuration YAMLs validator.yaml, fullnode.yaml, docker-compose.yaml and docker-compose-fullnode.yaml **do not contain any keys.** 

As noted earlier, a machine has an identity and a configuration. Hence:

- For a validator node, identity is defined in validator-**identity**.yaml and configuration is in validator.yaml. 
- For a validator fullnode, its identity is defined in validator-full-node-**identity**.yaml and its configuration is defined in fullnode.yaml.

### Persona

#### Identity and configuration

A persona has a single YAML that combines the persona’s identity and configuration information. For example, for the three personas, owner, operator and voter:

- An owner's identity-configuration is defined in **owner.yaml**. The owner.yaml contains the public keys and blockchain account addresses for owner, operator and voter, and some owner-specific configuration such as stake amount and commission percentage. 
- An operator’s identity-configuration is defined in **operator.yaml**. The operator.yaml contains public keys and blockchain account address for the operator and some machine configuration information plus a consensus public key and consensus proof of possession key. **Only the operator has the consensus keys.** Neither the owner nor the voter has the consensus keys. 
- A voter's identity-configuration, i.e., voter.yaml, does not exist. 

## Description of identity YAMLs

This section explains the following key and identity YAML files that are generated during the deployment of a validator node:

- `public-keys.yaml`.
- `private-keys.yaml`.
- `validator-identity.yaml`.
- `validator-full-node-identity.yaml`.

The following command is used to generate the above key and identity YAMLs. See, for example, [Step 10 while using AWS to deploy the validator node](./validator-node/operator/running-validator-node/using-aws.md), or in [Step 10 while using GCP](./validator-node/operator/running-validator-node/using-gcp.md). 

```bash
aptos genesis generate-keys --output-dir ~/$WORKSPACE/keys
```

See below a diagram that shows how the identities for the validator node and validator fullnode are derived from the private and public keys:

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/key-yamls.svg'),
    dark: useBaseUrl('/img/docs/key-yamls-dark.svg'),
  }}
/>

### public-keys.yaml

#### Example

Click below to see an example YAML configuration:
<details>
<summary>public-keys.yaml</summary>

```yaml
---
account_address: a5a643aa695fc5f34927386c8d767cddcc0607933f40c89a7ad78de7804965b8
account_public_key: "0x9ccfc50f334064e1b24455029a5bc1646a2c4dd2b1433de1364470692ba6b99b"
consensus_public_key: "0xa7e8334381d9f80d33d70da543aea22c87fe9862ab7df5cbef9ee11b5285b89c56e0e7a3a78c1561833b2d6fa4d9d4bf"
consensus_proof_of_possession: "0xa51dfd1734e581df99c4c637324ee38c3e48e51c61c1e1dd03bd5a84cf1cd5b2fa00e976b9a9ea0e0908f0d53085318c03f24de3ebf86b07ff883effe0142e0d3f24c7c1e36dd198ea4d8eb6f5c5a2f3a188de22720bd1914a9effa6f595de38"
full_node_network_public_key: "0xa6845691a00d6cfdaa9823c4d12b2b5e13d2ecfdc3049d0f2838c805bfd01633"
validator_network_public_key: "0x71f2642aeaa6cbfacf75663cf14d2f6e9e1bd890f9bc1c96900fd225cce01836"
```
 
</details>

#### Description

| public-keys.yaml | Description |
| --- | --- |
| account_address |The Aptos blockchain account address for the operator, i.e., the persona who deploys the validator node.  |
| account_public_key | The public key associated with the blockchain account. |
| consensus_public_key | Used only by the operator for validation purpose. |
| consensus_proof_of_possession | Used only by the operator for validation purpose. |
| full_node_network_public_key | The public key for the fullnode by which a VFN (validator fullnode) or a PFN (public fullnode) is identified in the Aptos network. |
| validator_network_public_key | The public key for the validator node and by which the validator node is identified in the Aptos network. |

### private-keys.yaml

#### Example

Click below to see an example YAML configuration:
<details>
<summary>private-keys.yaml</summary>

    
```yaml
---
account_address: a5a643aa695fc5f34927386c8d767cddcc0607933f40c89a7ad78de7804965b8
account_private_key: "0x80478d60a52f54a88e7095abf48b1f4294a335b30f1066cd73768b9b789e833f"
consensus_private_key: "0x4aedda33ef3fd71243eb2a926307d8826c95b9939f88e753d62d9bc577e99916"
full_node_network_private_key: "0x689c11c6e5405219b5eae1312086c801e3a044946afc74429e5157b46fb65b61"
validator_network_private_key: "0xa03ec46b24f2f1066d7980dc13b4baf722ba60c367e498e47a657ba0815adb58"
```

</details>

#### Description

| private-keys.yaml | Description |
| --- | --- |
| account_address | The Aptos blockchain account address for the operator, i.e., the persona who deploys the validator node. |
| account_private_key | The private key associated with the blockchain account. |
| consensus_private_key | The consensus private key, used only by the operator for validation purpose and for rotating the consensus key.|
| full_node_network_private_key |The private key for the fullnode. Whoever holds this private key will be able to establish the ownership of the VFN and PFN in the Aptos network. |
| validator_network_private_key | The private key for the validator node. Whoever holds this private key will be able to establish the ownership of the validator node in the Aptos network. |

### validator-identity.yaml

#### Example

Click below to see an example YAML configuration:

<details>
<summary>validator-identity.yaml</summary>
    

```yaml
---
account_address: a5a643aa695fc5f34927386c8d767cddcc0607933f40c89a7ad78de7804965b8
account_private_key: "0x80478d60a52f54a88e7095abf48b1f4294a335b30f1066cd73768b9b789e833f"
consensus_private_key: "0x4aedda33ef3fd71243eb2a926307d8826c95b9939f88e753d62d9bc577e99916"
network_private_key: "0xa03ec46b24f2f1066d7980dc13b4baf722ba60c367e498e47a657ba0815adb58"
```

</details>

#### Description

| validator-identity.yaml | Description |
| --- | --- |
| account_address | The Aptos blockchain account address for the operator, i.e., the persona who deploys the validator node. |
| account_private_key |The private key associated with the blockchain account. |
| consensus_private_key | The consensus private key, used only by the operator for validation purpose and for rotating the consensus key. |
| network_private_key | The private key for the validator node. Whoever holds this private key will be able to establish the ownership of the validator node in the Aptos network. |


### validator-full-node-identity.yaml

#### Example

Click below to see an example YAML configuration:

<details>
<summary>validator-full-node-identity.yaml</summary>

    
```yaml
---
account_address: a5a643aa695fc5f34927386c8d767cddcc0607933f40c89a7ad78de7804965b8
network_private_key: "0x689c11c6e5405219b5eae1312086c801e3a044946afc74429e5157b46fb65b61"
```

</details>
    

#### Description

| validator-full-node-identity.yaml | Description |
| --- | --- |
| account_address | The Aptos blockchain account address for the operator, i.e., the persona who deploys the validator node. |
| network_private_key | The private key for the fullnode. Whoever holds this private key will be able to establish the ownership of the VFN and PFN in the Aptos network. |




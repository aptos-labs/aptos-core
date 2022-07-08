# Forge 

Forge is a e2e testing framework built on top of the `aptos-node-testnet` kubernetes setup. It requires certain testnet configuration
for it to work. The below helm values must be set:

```
  aptos_node_helm_values_forge_override = {
    // no VFNs in forge for now
    fullnode = {
      groups = []
    }
    // make all services internal ClusterIP and open all ports
    service = {
      validator = {
        external = {
          type = "ClusterIP"
        }
        enableRestApi     = true
        enableMetricsPort = true
      }
      fullnode = {
        external = {
          type = "ClusterIP"
        }
        enableRestApi     = true
        enableMetricsPort = true
      }
    }
  }

  genesis_helm_values_forge_override = {
    chain = {
      # this key is hard-coded into forge. see:
      # testsuite/forge/src/backend/k8s/mod.rs
      root_key = "0x48136DF3174A3DE92AFDB375FFE116908B69FF6FAB9B1410E548A33FEA1D159D"
    }
  }
```

One can do the following to merge the above necessary Forge helm value overrides with custom helm values set for your deployment:

```
# merge the overrides with your own custom helm values
module "aptos-node-helm-values-deepmerge" {
  # https://registry.terraform.io/modules/Invicton-Labs/deepmerge/null/0.1.5
  source = "Invicton-Labs/deepmerge/null"
  maps = [
    local.aptos_node_helm_values_forge_override,
    local.YOUR_APTOS_NODE_HELM_VALUES,
  ]
}

# invoke testnet, specifying aptos_node_helm_values with the above merged values
module "aptos-testnet" {
  source = "git@github.com:aptos-labs/aptos-core.git//terraform/testnet?ref=main"
  ...
  aptos_node_helm_values = module.aptos-node-helm-values-deepmerge.merged

}
```

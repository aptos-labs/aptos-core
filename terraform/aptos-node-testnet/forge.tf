
# Forge testing overrides
locals {
  # Forge assumes the chain_id is 4
  chain_id = var.enable_forge ? 4 : var.chain_id

  aptos_node_helm_values_forge_override = {
    // Hit validators directly in Forge, rather than using HAProxy
    validator = {
      enableNetworkPolicy = false
    }
    haproxy = {
      enabled = false
    }
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
}

# helm value override merging with forge
module "aptos-node-helm-values-deepmerge" {
  # https://registry.terraform.io/modules/Invicton-Labs/deepmerge/null/0.1.5
  source = "Invicton-Labs/deepmerge/null"
  maps = [
    var.enable_forge ? tomap(local.aptos_node_helm_values_forge_override) : {},
    var.aptos_node_helm_values,
  ]
}

module "genesis-helm-values-deepmerge" {
  # https://registry.terraform.io/modules/Invicton-Labs/deepmerge/null/0.1.5
  source = "Invicton-Labs/deepmerge/null"
  maps = [
    var.enable_forge ? tomap(local.genesis_helm_values_forge_override) : {},
    var.genesis_helm_values,
  ]
}

resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = "${path.module}/../helm/forge"
  max_history = 2
  wait        = false

  values = [
    jsonencode({
      forge = {
        image = {
          tag = var.image_tag
        }
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}


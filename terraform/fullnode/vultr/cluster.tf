resource "vultr_kubernetes" "k8" {
  region  = var.fullnode_region
  label   = "aptos-${terraform.workspace}"
  version = "v1.23.5+3"

  node_pools {
    node_quantity = var.num_fullnodes
    plan          = var.machine_type
    label         = "aptos-fullnode"
  }
}
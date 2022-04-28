resource "vultr_kubernetes" "k8" {
  region = "ams"
  label     = "aptos-devnet-ams"
  version = "v1.23.5+3"

  node_pools {
    node_quantity = 1
    plan = "vc2-4c-8gb"
    label = "node"
  }
}

resource "local_file" "kube_config" {
    content  = base64decode(vultr_kubernetes.k8.kube_config)
    filename = "${path.module}/vultr_kube_config.yml"
}
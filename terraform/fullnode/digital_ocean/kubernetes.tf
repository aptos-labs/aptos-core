provider "kubernetes" {
  host                   = digitalocean_kubernetes_cluster.aptos.endpoint
  cluster_ca_certificate = base64decode(digitalocean_kubernetes_cluster.aptos.kube_config[0].cluster_ca_certificate)
  token                  = digitalocean_kubernetes_cluster.aptos.kube_config[0].token
}

resource "kubernetes_namespace" "aptos" {
  metadata {
    name = var.k8s_namespace
  }
}

provider "helm" {
  kubernetes {
    host                   = digitalocean_kubernetes_cluster.aptos.endpoint
    cluster_ca_certificate = base64decode(digitalocean_kubernetes_cluster.aptos.kube_config[0].cluster_ca_certificate)
    token                  = digitalocean_kubernetes_cluster.aptos.kube_config[0].token
  }
}

locals {
  fullnode_helm_chart_path = "${path.module}/../../helm/fullnode"
}

resource "helm_release" "fullnode" {
  count            = var.num_fullnodes
  name             = "${terraform.workspace}${count.index}"
  chart            = local.fullnode_helm_chart_path
  max_history      = 100
  wait             = false
  namespace        = var.k8s_namespace
  create_namespace = true

  values = [
    jsonencode({
      chain = {
        era = var.era
      }
      image = {
        tag = var.image_tag
      }
      nodeSelector = {
        "doks.digitalocean.com/node-pool" = digitalocean_kubernetes_cluster.aptos.node_pool[0].name
      }
      storageClass = {
        class = "do-block-storage"
      }
      service = {
        type = "LoadBalancer"
      }
      storage = {
        size = "100Gi"
      }
    }),
    jsonencode(var.fullnode_helm_values),
    jsonencode(var.fullnode_helm_values_list == {} ? {} : var.fullnode_helm_values_list[count.index]),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.fullnode_helm_chart_path, "**") : filesha1("${local.fullnode_helm_chart_path}/${f}")]))
  }
}

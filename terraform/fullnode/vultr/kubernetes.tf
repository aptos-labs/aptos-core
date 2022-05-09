provider "kubernetes" {
  host = yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).clusters[0].cluster["server"]
  cluster_ca_certificate = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).clusters[0].cluster["certificate-authority-data"])
  client_certificate = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).users[0].user["client-certificate-data"])
  client_key = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).users[0].user["client-key-data"])
}

resource "kubernetes_namespace" "aptos" {
  metadata {
    name = var.k8s_namespace
  }
}

provider "helm" {
  kubernetes {
    host = yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).clusters[0].cluster["server"]
    cluster_ca_certificate = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).clusters[0].cluster["certificate-authority-data"])
    client_certificate = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).users[0].user["client-certificate-data"])
    client_key = base64decode(yamldecode(base64decode(vultr_kubernetes.k8.kube_config)).users[0].user["client-key-data"])
  }
}

resource "helm_release" "fullnode" {
  count            = var.num_fullnodes
  name             = "${terraform.workspace}${count.index}"
  chart            = "${path.module}/../../helm/fullnode"
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
        "vke.vultr.com/node-pool" = "aptos-fullnode"
      }
      storage = {
        class = var.block_storage_class
      }
      service = {
        type = "LoadBalancer"
      }
    }),
    jsonencode(var.fullnode_helm_values),
    jsonencode(var.fullnode_helm_values_list == {} ? {} : var.fullnode_helm_values_list[count.index]),
  ]

  set {
    name  = "timestamp"
    value = var.helm_force_update ? timestamp() : ""
  }
}


provider "kubernetes" {
  config_path = local_file.kube_config.filename
}

resource "kubernetes_namespace" "aptos" {
  metadata {
    name = var.k8s_namespace
  }
}

resource "kubernetes_storage_class" "ssd" {
  metadata {
    name = "ssd"
  }
  storage_provisioner = "kubernetes.io/gce-pd"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "pd-ssd"
  }
}

provider "helm" {
  kubernetes {
    config_path = local_file.kube_config.filename
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
        era  = var.era
      }
      image = {
        tag = var.image_tag
      }
      nodeSelector = {
        "vke.vultr.com/node-pool" = "node"
      }
      storage = {
        class = kubernetes_storage_class.ssd.metadata[0].name
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


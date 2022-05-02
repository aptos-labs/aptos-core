provider "kubernetes" {
  host                   = "https://${google_container_cluster.aptos.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
  token                  = data.google_client_config.provider.access_token
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
    host                   = "https://${google_container_cluster.aptos.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/aptos-node"
  max_history = 100
  wait        = false

  values = [
    jsonencode({
      imageTag = var.image_tag
      chain = {
      era        = var.era
      chain_id   = var.chain_id
      chain_name = var.chain_name
      }
      validator = {
        name = var.validator_name
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
        }
        nodeSelector = {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.validators.name
        }
        tolerations = [{
          key    = google_container_node_pool.validators.node_config[0].taint[0].key
          value  = google_container_node_pool.validators.node_config[0].taint[0].value
          effect = "NoExecute"
        }]
      }
      fullnode = {
        storage = {
          class = "standard"
        }
        nodeSelector = {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.validators.name
        }
        tolerations = [{
          key    = google_container_node_pool.validators.node_config[0].taint[0].key
          value  = google_container_node_pool.validators.node_config[0].taint[0].value
          effect = "NoExecute"
        }]
      }
    }),
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  set {
    name  = "timestamp"
    value = var.helm_force_update ? timestamp() : ""
  }
}

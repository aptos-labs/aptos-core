provider "kubernetes" {
  host                   = "https://${google_container_cluster.diem.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.diem.master_auth[0].cluster_ca_certificate)
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
    host                   = "https://${google_container_cluster.diem.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.diem.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

locals {
  vault          = {}
  network_values = "${path.module}/../helm/values/${split("-", terraform.workspace)[0]}.yaml"
}

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = var.helm_chart
  max_history = 100
  wait        = false

  values = [
    jsonencode({
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
      safetyrules = {
        nodeSelector = {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.trusted.name
        }
        tolerations = [{
          key    = google_container_node_pool.trusted.node_config[0].taint[0].key
          value  = google_container_node_pool.trusted.node_config[0].taint[0].value
          effect = "NoExecute"
        }]
      }
      keymanager = {
        nodeSelector = {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.trusted.name
        }
        tolerations = [{
          key    = google_container_node_pool.trusted.node_config[0].taint[0].key
          value  = google_container_node_pool.trusted.node_config[0].taint[0].value
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
      haproxy = {
        nodeSelector = {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.validators.name
        }
        tolerations = [{
          key    = google_container_node_pool.validators.node_config[0].taint[0].key
          value  = google_container_node_pool.validators.node_config[0].taint[0].value
          effect = "NoExecute"
        }]
      }
      monitoring = {
        fullKubernetesScrape = true
        prometheus = {
          storage = {
            class = "standard"
          }
        }
      }
      backup = {
        config = {
          location = "gcs"
          gcs = {
            bucket = google_storage_bucket.backup.name
          }
        }
        serviceAccount = {
          annotations = {
            "iam.gke.io/gcp-service-account" = google_service_account.backup.email
          }
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
      restore = {
        config = {
          location = "gcs"
          gcs = {
            bucket = google_storage_bucket.backup.name
          }
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
      vault = local.vault
    }),
    fileexists(local.network_values) ? file(local.network_values) : "{}",
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  set {
    name  = "timestamp"
    value = var.helm_force_update ? timestamp() : ""
  }
}

resource "local_file" "kubernetes" {
  filename = "${terraform.workspace}-kubernetes.json"
  content = jsonencode({
    kubernetes_host        = "https://${google_container_cluster.diem.private_cluster_config[0].private_endpoint}"
    kubernetes_ca_cert     = base64decode(google_container_cluster.diem.master_auth[0].cluster_ca_certificate)
    issuer                 = "https://container.googleapis.com/v1/${google_container_cluster.diem.id}"
    service_account_prefix = "${terraform.workspace}-diem-validator"
    pod_cidrs              = [google_container_cluster.diem.cluster_ipv4_cidr]
  })
  file_permission = "0644"
}

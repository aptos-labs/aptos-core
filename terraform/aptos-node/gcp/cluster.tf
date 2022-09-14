resource "google_container_cluster" "aptos" {
  provider = google-beta
  name     = "aptos-${terraform.workspace}"
  location = local.zone
  network  = google_compute_network.aptos.id

  remove_default_node_pool = true
  initial_node_count       = 1
  logging_service          = "logging.googleapis.com/kubernetes"
  monitoring_service       = "none"

  release_channel {
    channel = "REGULAR"
  }

  master_auth {
    client_certificate_config {
      issue_client_certificate = false
    }
  }

  master_authorized_networks_config {
    dynamic "cidr_blocks" {
      for_each = var.k8s_api_sources
      content {
        cidr_block = cidr_blocks.value
      }
    }
  }

  private_cluster_config {
    enable_private_nodes    = true
    enable_private_endpoint = false
    master_ipv4_cidr_block  = "172.16.0.0/28"
  }

  ip_allocation_policy {
    cluster_ipv4_cidr_block = ""
  }

  workload_identity_config {
    workload_pool = "${var.project}.svc.id.goog"
  }

  addons_config {
    network_policy_config {
      disabled = false
    }
  }

  network_policy {
    enabled  = true
    provider = "CALICO"
  }

  pod_security_policy_config {
    enabled = true
  }
}

resource "google_container_node_pool" "utilities" {
  provider   = google-beta
  name       = "utilities"
  location   = local.zone
  cluster    = google_container_cluster.aptos.name
  node_count = lookup(var.node_pool_sizes, "utilities", var.utility_instance_num)

  node_config {
    machine_type    = var.utility_instance_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 20
    service_account = google_service_account.gke.email
    tags            = ["utilities"]
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    # if the NodeGroup should be tainted, then create the below dynamic block
    dynamic "taint" {
      for_each = var.utility_instance_enable_taint ? ["utilities"] : []
      content {
        key    = "aptos.org/nodepool"
        value  = each.key
        effect = "NO_EXECUTE"
      }
    }
  }
}

resource "google_container_node_pool" "validators" {
  provider   = google-beta
  name       = "validators"
  location   = local.zone
  cluster    = google_container_cluster.aptos.name
  node_count = lookup(var.node_pool_sizes, "validators", var.validator_instance_num)

  node_config {
    machine_type    = var.validator_instance_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 20
    service_account = google_service_account.gke.email
    tags            = ["validators"]
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    # if the NodeGroup should be tainted, then create the below dynamic block
    dynamic "taint" {
      for_each = var.validator_instance_enable_taint ? ["validators"] : []
      content {
        key    = "aptos.org/nodepool"
        value  = each.key
        effect = "NO_EXECUTE"
      }
    }
  }
}

locals {
  location = var.zone == "" ? var.region : "${var.region}-${var.zone}"
}

resource "google_container_cluster" "velor" {
  provider       = google-beta
  name           = "velor-${terraform.workspace}"
  location       = local.location
  node_locations = var.node_locations
  network        = google_compute_network.velor.id

  remove_default_node_pool = true
  initial_node_count       = 1

  cost_management_config {
    enabled = true
  }

  release_channel {
    channel = "STABLE"
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
      disabled = true
    }
  }

  network_policy {
    enabled = false
  }

  pod_security_policy_config {
    enabled = false
  }

  dynamic "dns_config" {
    for_each = var.enable_clouddns ? ["clouddns"] : []
    content {
      cluster_dns       = "CLOUD_DNS"
      cluster_dns_scope = "CLUSTER_SCOPE"
    }
  }

  monitoring_config {
    managed_prometheus {
      enabled = true
    }
    # Enable all components.
    enable_components = [
      "APISERVER",
      "CONTROLLER_MANAGER",
      "DAEMONSET",
      "DEPLOYMENT",
      "HPA",
      "POD",
      "SCHEDULER",
      "STATEFULSET",
      "STORAGE",
      "SYSTEM_COMPONENTS",
    ]
  }

  dynamic "cluster_autoscaling" {
    for_each = var.gke_enable_node_autoprovisioning ? [1] : []
    content {
      enabled             = var.gke_enable_node_autoprovisioning
      autoscaling_profile = var.gke_autoscaling_profile

      dynamic "resource_limits" {
        for_each = {
          "cpu"    = var.gke_node_autoprovisioning_max_cpu
          "memory" = var.gke_node_autoprovisioning_max_memory
        }
        content {
          resource_type = resource_limits.key
          minimum       = 1
          maximum       = resource_limits.value
        }
      }

      auto_provisioning_defaults {
        service_account = google_service_account.gke.email
        oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]
        disk_size       = var.default_disk_size_gb
        disk_type       = var.default_disk_type
        management {
          auto_upgrade = true
          auto_repair  = true
        }
        shielded_instance_config {
          enable_integrity_monitoring = true
          enable_secure_boot          = true
        }
      }
    }
  }

  node_pool_defaults {
    node_config_defaults {
      gcfs_config {
        enabled = var.enable_image_streaming
      }
    }
  }

  maintenance_policy {
    dynamic "recurring_window" {
      for_each = var.gke_maintenance_policy.recurring_window != null ? [1] : []
      content {
        start_time = var.gke_maintenance_policy.recurring_window.start_time
        end_time   = var.gke_maintenance_policy.recurring_window.end_time
        recurrence = var.gke_maintenance_policy.recurring_window.recurrence
      }
    }
  }

  lifecycle {
    ignore_changes = [
      private_cluster_config,
    ]
  }
  deletion_protection = false
}

resource "google_container_node_pool" "core" {
  count      = var.create_nodepools ? 1 : 0
  provider   = google-beta
  name       = "core"
  location   = local.location
  cluster    = google_container_cluster.velor.name
  node_count = lookup(var.node_pool_sizes, "core", null)

  node_config {
    machine_type    = var.core_instance_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = lookup(var.instance_disk_sizes, "core", var.default_disk_size_gb)
    service_account = google_service_account.gke.email
    tags            = ["core"]
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    shielded_instance_config {
      enable_integrity_monitoring = true
      enable_secure_boot          = true
    }

    # The core machine type is too small (<16G) to support image streaming.
    gcfs_config {
      enabled = false
    }

    gvnic {
      enabled = true
    }

    kubelet_config {
      cpu_manager_policy = "none"
    }
  }

  autoscaling {
    min_node_count = 0
    max_node_count = var.gke_autoscaling_max_node_count
  }
}

resource "google_container_node_pool" "utilities" {
  count      = var.create_nodepools ? 1 : 0
  provider   = google-beta
  name       = "utilities"
  location   = local.location
  cluster    = google_container_cluster.velor.name
  node_count = lookup(var.node_pool_sizes, "utilities", null)

  node_config {
    machine_type    = var.utility_instance_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = lookup(var.instance_disk_sizes, "utilities", var.default_disk_size_gb)
    service_account = google_service_account.gke.email
    tags            = ["utilities"]
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    shielded_instance_config {
      enable_integrity_monitoring = true
      enable_secure_boot          = true
    }

    gvnic {
      enabled = true
    }

    kubelet_config {
      cpu_manager_policy = "none"
    }
    linux_node_config {
      sysctls = var.nodepool_sysctls
    }

    # if the NodeGroup should be tainted, then create the below dynamic block
    dynamic "taint" {
      for_each = var.utility_instance_enable_taint ? ["utilities"] : []
      content {
        key    = "velor.org/nodepool"
        value  = taint.value
        effect = "NO_EXECUTE"
      }
    }
  }

  autoscaling {
    min_node_count = 0
    max_node_count = var.gke_autoscaling_max_node_count
  }
}

resource "google_container_node_pool" "fullnodes" {
  count      = var.create_nodepools ? 1 : 0
  provider   = google-beta
  name       = "fullnodes"
  location   = local.location
  cluster    = google_container_cluster.velor.name
  node_count = lookup(var.node_pool_sizes, "fullnodes", null)

  node_config {
    machine_type    = var.fullnode_instance_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = lookup(var.instance_disk_sizes, "fullnodes", var.default_disk_size_gb)
    service_account = google_service_account.gke.email
    tags            = ["fullnodes"]
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    shielded_instance_config {
      enable_integrity_monitoring = true
      enable_secure_boot          = true
    }

    gvnic {
      enabled = true
    }

    kubelet_config {
      cpu_manager_policy = "static"
    }
    linux_node_config {
      sysctls = var.nodepool_sysctls
    }

    # if the NodeGroup should be tainted, then create the below dynamic block
    dynamic "taint" {
      for_each = var.fullnode_instance_enable_taint ? ["fullnodes"] : []
      content {
        key    = "velor.org/nodepool"
        value  = taint.value
        effect = "NO_EXECUTE"
      }
    }
  }

  autoscaling {
    min_node_count = 0
    max_node_count = var.gke_autoscaling_max_node_count
  }
}

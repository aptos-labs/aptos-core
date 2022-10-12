resource "google_service_account" "k8s-gcp-integrations" {
  account_id = "${terraform.workspace}-pfn-gcp"
}

resource "google_project_iam_member" "k8s-gcp-integrations-dns" {
  project = local.zone_project
  role    = "roles/dns.admin"
  member  = "serviceAccount:${google_service_account.k8s-gcp-integrations.email}"
}

resource "google_service_account_iam_binding" "k8s-gcp-integrations" {
  service_account_id = google_service_account.k8s-gcp-integrations.name
  role               = "roles/iam.workloadIdentityUser"
  members            = ["serviceAccount:${google_container_cluster.aptos.workload_identity_config[0].workload_pool}[kube-system/k8s-gcp-integrations]"]
}

resource "kubernetes_service_account" "k8s-gcp-integrations" {
  metadata {
    name      = "k8s-gcp-integrations"
    namespace = "kube-system"
    annotations = {
      "iam.gke.io/gcp-service-account" = google_service_account.k8s-gcp-integrations.email
    }
  }
}

data "google_dns_managed_zone" "pfn" {
  count   = var.zone_name != "" ? 1 : 0
  name    = var.zone_name
  project = local.zone_project
}

locals {
  dns_prefix = var.workspace_dns ? "${local.workspace_name}.${var.dns_prefix_name}." : "${var.dns_prefix_name}."
  domain     = var.zone_name != "" ? "${local.dns_prefix}${data.google_dns_managed_zone.pfn[0].dns_name}" : null
}

resource "helm_release" "external-dns" {
  count       = var.zone_name != "" ? 1 : 0
  name        = "external-dns"
  repository  = "https://kubernetes-sigs.github.io/external-dns"
  chart       = "external-dns"
  version     = "1.11.0"
  namespace   = "kube-system"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      serviceAccount = {
        create = false
        name   = kubernetes_service_account.k8s-gcp-integrations.metadata[0].name
      }
      provider      = "google"
      domainFilters = var.zone_name != "" ? [data.google_dns_managed_zone.pfn[0].dns_name] : []
      extraArgs = [
        "--google-project=${local.zone_project}",
        "--txt-owner-id=${terraform.workspace}",
        "--txt-prefix=aptos",
      ]
    })
  ]
}

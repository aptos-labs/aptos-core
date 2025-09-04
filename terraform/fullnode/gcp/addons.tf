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
  members            = ["serviceAccount:${google_container_cluster.velor.workload_identity_config[0].workload_pool}[kube-system/k8s-gcp-integrations]"]
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
  zone_project = var.zone_project != "" ? var.zone_project : var.project
  dns_prefix   = var.workspace_dns ? "${local.workspace_name}.${var.dns_prefix_name}." : "${var.dns_prefix_name}."
  domain       = var.zone_name != "" ? trimsuffix("${local.dns_prefix}${data.google_dns_managed_zone.pfn[0].dns_name}", ".") : null
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
        "--txt-prefix=velor",
      ]
    })
  ]
}

resource "helm_release" "pfn-addons" {
  depends_on = [
    helm_release.fullnode
  ]
  name        = "pfn-addons"
  chart       = local.pfn_addons_helm_chart_path
  max_history = 10
  wait        = false
  namespace   = var.k8s_namespace

  values = [
    jsonencode({
      service = {
        domain = local.domain
      }
      ingress = {
        class                           = "gce"
        backend_http2                   = var.backend_http2
        gce_managed_certificate         = var.create_google_managed_ssl_certificate ? "velor-${local.workspace_name}-ingress" : null
        gce_managed_certificate_domains = var.create_google_managed_ssl_certificate ? join(",", distinct(concat([local.domain], var.tls_sans))) : ""
        # loadBalancerSourceRanges = var.client_sources_ipv4 # not supported yet
      }
    }),
    jsonencode(var.pfn_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.pfn_addons_helm_chart_path, "**") : filesha1("${local.pfn_addons_helm_chart_path}/${f}")]))
  }
}

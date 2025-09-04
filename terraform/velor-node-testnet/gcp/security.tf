# Security-related resources

locals {
  privileged_pss_labels = {
    "pod-security.kubernetes.io/audit"   = "baseline"
    "pod-security.kubernetes.io/warn"    = "baseline"
    "pod-security.kubernetes.io/enforce" = "privileged"
  }
  baseline_pss_labels = {
    "pod-security.kubernetes.io/audit"   = "restricted"
    "pod-security.kubernetes.io/warn"    = "restricted"
    "pod-security.kubernetes.io/enforce" = "baseline"
  }
  restricted_pss_labels = {
    "pod-security.kubernetes.io/enforce" = "restricted"
  }
}

resource "kubernetes_labels" "pss-chaos-mesh" {
  count       = var.enable_forge ? 1 : 0
  api_version = "v1"
  kind        = "Namespace"
  metadata {
    name = "chaos-mesh"
  }
  labels     = local.privileged_pss_labels
  depends_on = [kubernetes_namespace.chaos-mesh]
}

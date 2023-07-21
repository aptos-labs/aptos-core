# Security-related resources

locals {
  # Enforce "privileged" PSS (i.e. allow everything), but warn about
  # infractions of "baseline" profile
  privileged_pss_labels = {
    "pod-security.kubernetes.io/audit"   = "baseline"
    "pod-security.kubernetes.io/warn"    = "baseline"
    "pod-security.kubernetes.io/enforce" = "privileged"
  }
}

resource "kubernetes_labels" "pss-default" {
  api_version = "v1"
  kind        = "Namespace"
  metadata {
    name = "default"
  }
  labels = local.privileged_pss_labels
}

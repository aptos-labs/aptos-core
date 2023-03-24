# Security-related resources

data "kubernetes_all_namespaces" "all" {}

# FIXME: Remove when migrating to K8s 1.25
resource "kubernetes_role_binding" "disable-psp" {
  for_each = toset(var.kubernetes_version <= "1.24" ? data.kubernetes_all_namespaces.all.namespaces : [])
  metadata {
    name      = "privileged-psp"
    namespace = each.value
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = "eks:podsecuritypolicy:privileged"
  }

  subject {
    api_group = "rbac.authorization.k8s.io"
    kind      = "Group"
    name      = "system:serviceaccounts:${each.value}"
  }
}

locals {
  baseline_pss_labels = {
    "pod-security.kubernetes.io/audit"           = "baseline"
    "pod-security.kubernetes.io/audit-version"   = "v${var.kubernetes_version}"
    "pod-security.kubernetes.io/warn"            = "baseline"
    "pod-security.kubernetes.io/warn-version"    = "v${var.kubernetes_version}"
    "pod-security.kubernetes.io/enforce"         = "privileged"
    "pod-security.kubernetes.io/enforce-version" = "v${var.kubernetes_version}"
  }
}

resource "kubernetes_labels" "pss-default" {
  api_version = "v1"
  kind        = "Namespace"
  metadata {
    name = "default"
  }
  labels = local.baseline_pss_labels
}

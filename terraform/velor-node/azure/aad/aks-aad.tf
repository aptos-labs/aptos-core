variable "k8s_admin_groups" {
  description = "List of AD Group IDs to configure as Kubernetes admins"
  type        = list(string)
}

resource "kubernetes_cluster_role" "debug" {
  metadata {
    name = "debug"
  }

  rule {
    api_groups = [""]
    resources  = ["pods/portforward", "pods/exec"]
    verbs      = ["create"]
  }
}

resource "kubernetes_role_binding" "aad-debuggers" {
  count = min(length(var.k8s_debugger_groups), 1)

  metadata {
    name = "aad-debuggers"
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = kubernetes_cluster_role.debug.metadata[0].name
  }

  dynamic "subject" {
    for_each = var.k8s_debugger_groups
    content {
      kind = "Group"
      name = subject.value
    }
  }
}

resource "kubernetes_role_binding" "aad-viewers" {
  count = min(length(var.k8s_viewer_groups) + length(var.k8s_debugger_groups), 1)

  metadata {
    name = "aad-viewers"
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = "view"
  }

  dynamic "subject" {
    for_each = var.k8s_viewer_groups
    content {
      kind = "Group"
      name = subject.value
    }
  }
  dynamic "subject" {
    for_each = var.k8s_debugger_groups
    content {
      kind = "Group"
      name = subject.value
    }
  }
}

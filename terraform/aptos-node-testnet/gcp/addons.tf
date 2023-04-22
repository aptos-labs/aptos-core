locals {
  chaos_mesh_helm_chart_path = "${path.module}/../../helm/chaos"
}

resource "kubernetes_namespace" "chaos-mesh" {
  metadata {
    annotations = {
      name = "chaos-mesh"
    }

    name = "chaos-mesh"
  }
}

resource "helm_release" "chaos-mesh" {
  name      = "chaos-mesh"
  namespace = kubernetes_namespace.chaos-mesh.metadata[0].name

  chart       = local.chaos_mesh_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      chaos-mesh = {
        chaosDaemon = {
          runtime    = "containerd"
          socketPath = "/run/containerd/containerd.sock"
          image = {
            repository = "aptos-internal/chaos-daemon"
            tag        = "latest"
          }
        },
        controllerManager = {
          image = {
            repository = "aptos-internal/chaos-mesh"
            tag        = "latest"
          }
        },
        dashboard = {
          image = {
            repository = "aptos-internal/chaos-dashboard"
            tag        = "latest"
          }
        }
        images = {
          registry = "us-west1-docker.pkg.dev/aptos-global"
        }
      }
    })
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.chaos_mesh_helm_chart_path, "**") : filesha1("${local.chaos_mesh_helm_chart_path}/${f}")]))
  }
}

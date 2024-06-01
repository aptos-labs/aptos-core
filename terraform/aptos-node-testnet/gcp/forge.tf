locals {
  forge_helm_chart_path = "${path.module}/../../helm/forge"
}
resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = local.forge_helm_chart_path
  max_history = 2
  wait        = false

  values = [
    jsonencode({
      forge = {
        image = {
          tag = var.image_tag
        }
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.forge_helm_chart_path, "**") : filesha1("${local.forge_helm_chart_path}/${f}")]))
  }
}


resource "kubernetes_secret" "grafana_credentials" {
  metadata {
    name      = "credentials"
    namespace = "grafana"
  }

  # Ignore changes to the data field to prevent replacing the manually updated password
  lifecycle {
    ignore_changes = [
      data,
    ]
  }

  # Create a placeholder password. This should be set manually in each cluster
  data = {
    password = base64encode("placeholder")
  }
}

resource "helm_release" "grafana_agent_flow" {
  name       = "grafana-agent-flow"
  repository = "https://grafana.github.io/helm-charts"
  chart      = "grafana-agent"
  version    = "0.37.0"
  namespace  = "grafana"

  values = [
    yamlencode({
      agent = {
        mode = "flow"
        configMap = {
          create  = true
          content = <<-EOT
          remote.kubernetes.secret "credentials" {
            namespace = "grafana"
            name = "credentials"
          }
          discovery.kubernetes "local_pods" {
            selectors {
              field = "spec.nodeName=" + env("HOSTNAME")
              role = "pod"
            }
            role = "pod"
          }
          discovery.relabel "specific_pods" {
            targets = discovery.kubernetes.local_pods.targets
            rule {
              action = "drop"
              regex = "Succeeded|Failed|Completed"
              source_labels = ["__meta_kubernetes_pod_phase"]
            }
            rule {
              action = "replace"
              source_labels = ["__meta_kubernetes_namespace"]
              target_label = "namespace"
            }
            rule {
              action = "replace"
              source_labels = ["__meta_kubernetes_pod_name"]
              target_label = "pod"
            }
            rule {
              action = "replace"
              source_labels = ["__meta_kubernetes_pod_node_name"]
              target_label = "node"
            }
            rule {
              action = "replace"
              source_labels = ["__meta_kubernetes_pod_container_name"]
              target_label = "container"
            }
            rule {
              action = "replace"
              regex = "(.*)@(.*)"
              replacement = "ebpf/$${1}/$${2}"
              separator = "@"
              source_labels = ["__meta_kubernetes_namespace", "__meta_kubernetes_pod_container_name"]
              target_label = "service_name"
            }
          }
          pyroscope.ebpf "instance" {
            forward_to = [pyroscope.write.endpoint.receiver]
            targets = discovery.relabel.specific_pods.output
            demangle = "full"
          }
          pyroscope.write "endpoint" {
            endpoint {
              url = "https://profiles-prod-003.grafana.net"
                basic_auth {
                  username = "340750"
                  password = remote.kubernetes.secret.credentials.data["password"]
              }
            }
          }
          EOT
        }
        securityContext = {
          privileged = true
          runAsGroup = 0
          runAsUser  = 0
        }
      }
      controller = {
        hostPID = true
      }
    })
  ]
}

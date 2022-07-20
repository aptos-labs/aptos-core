locals {
  indexer_helm_chart_path = "${path.module}/../../helm/indexer"
}

resource "helm_release" "indexer" {
  name        = "indexer-${local.workspace_name}"
  chart       = local.indexer_helm_chart_path
  max_history = 2
  wait        = false

  values = [
    jsonencode({
      nodeUrl = var.node_url
      indexer = {
        image = {
          tag = var.image_tag
        }
      }
      nginx = {
        enabled = true
        upstream = {
          main = "${aws_db_instance.indexer.address}:5432"
        }
      }
      serviceAccount = {
        annotations = {
          "eks.amazonaws.com/role-arn" = aws_iam_role.indexer.arn
        }
      }
      prometheus-postgres-exporter = {
        config = {
          datasourceSecret = {
            name = kubernetes_secret.indexer_credentials.metadata[0].name
            key  = "pg_db_uri"
          }
        }
      }
    }),
    jsonencode(var.indexer_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.indexer_helm_chart_path, "**") : filesha1("${local.indexer_helm_chart_path}/${f}")]))
  }
}

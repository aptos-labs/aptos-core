resource "helm_release" "indexer" {
  name        = "indexer"
  chart       = "${path.module}/../../helm/indexer"
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
    }),
    jsonencode(var.indexer_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

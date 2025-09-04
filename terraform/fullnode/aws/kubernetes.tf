locals {
  pfn_addons_helm_chart_path = "${path.module}/../../helm/pfn-addons"
  fullnode_helm_chart_path   = "${path.module}/../../helm/fullnode"
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"
}

resource "helm_release" "fullnode" {
  count       = var.num_fullnodes
  name        = "pfn${count.index}"
  chart       = local.fullnode_helm_chart_path
  max_history = 10
  wait        = false

  depends_on = [module.eks]

  values = [
    jsonencode({
      imageTag     = var.image_tag
      manageImages = var.manage_via_tf # if we're managing the entire deployment via terraform, override the images as well
      chain = {
        era  = var.era
        name = var.chain_name
      }
      image = {
        tag = local.image_tag
      }
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "fullnode"
      }
      storage = {
        class = var.fullnode_storage_class
        size  = var.fullnode_storage_size
      }
      service = {
        type = "LoadBalancer"
        annotations = {
          "service.beta.kubernetes.io/aws-load-balancer-type" = "nlb"
          "external-dns.alpha.kubernetes.io/hostname"         = "pfn${count.index}.${local.domain}"
          "alb.ingress.kubernetes.io/healthcheck-path"        = "/v1/-/healthy"
        }
      }
      backup = {
        enable = count.index == var.backup_fullnode_index ? var.enable_backup : false
        config = {
          location = "s3"
          s3 = {
            bucket = aws_s3_bucket.backup.bucket
          }
        }
      }
      restore = {
        config = {
          location = "s3"
          s3 = {
            bucket = aws_s3_bucket.backup.bucket
          }
        }
      }
      serviceAccount = {
        annotations = {
          "eks.amazonaws.com/role-arn" = aws_iam_role.backup.arn
        }
      }
    }),
    jsonencode(var.fullnode_helm_values),
    jsonencode(var.fullnode_helm_values_list == {} ? {} : var.fullnode_helm_values_list[count.index]),
  ]

  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.fullnode_helm_chart_path, "**") : filesha1("${local.fullnode_helm_chart_path}/${f}")]))
    }
  }
}

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "velor-monitoring"
  chart       = local.monitoring_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      chain = {
        name = var.chain_name
      }
      fullnode = {
        name = var.fullnode_name
      }
      service = {
        domain = local.domain
      }
      monitoring = {
        prometheus = {
          storage = {
            class = "gp3"
          }
        }
      }
      kube-state-metrics = {
        enabled = var.enable_kube_state_metrics
      }
      prometheus-node-exporter = {
        enabled = var.enable_prometheus_node_exporter
      }
    }),
    jsonencode(var.monitoring_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.monitoring_helm_chart_path, "**") : filesha1("${local.monitoring_helm_chart_path}/${f}")]))
  }
}

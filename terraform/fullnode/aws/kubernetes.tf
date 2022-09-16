locals {
  pfn_helm_chart_path        = "${path.module}/fullnode"
  pfn_logger_helm_chart_path = "${path.module}/../../helm/logger"
  fullnode_helm_chart_path   = "${path.module}/../../helm/fullnode"
}

resource "helm_release" "pfn" {
  name        = "aptos"
  chart       = local.pfn_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      imageTag = local.image_tag
      service = {
        domain   = local.domain
        aws_tags = local.aws_tags
        fullnode = {
          numFullnodes             = var.num_fullnodes
          loadBalancerSourceRanges = var.client_sources_ipv4
        }
        monitoring = {
          loadBalancerSourceRanges = var.admin_sources_ipv4
        }
      }
      ingress = {
        acm_certificate          = var.zone_id != "" ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = var.client_sources_ipv4
      }
      monitoring = {
        prometheus = {
          storage = {
            class = "gp2"
          }
        }
      }
    }),
    jsonencode(var.pfn_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.pfn_helm_chart_path, "**") : filesha1("${local.pfn_helm_chart_path}/${f}")]))
  }
}

resource "helm_release" "fullnode" {
  count       = var.num_fullnodes
  name        = "pfn${count.index}"
  chart       = local.fullnode_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      chain = {
        era = var.era
      }
      image = {
        tag = local.image_tag
      }
      logging = {
        address = var.enable_pfn_logger ? "fullnode-pfn-aptos-logger:5044" : ""
      }
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "fullnode"
      }
      storage = {
        class = "gp2"
      }
      backup = {
        enable = count.index == 0 ? var.enable_backup : false
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

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.fullnode_helm_chart_path, "**") : filesha1("${local.fullnode_helm_chart_path}/${f}")]))
  }
}


resource "helm_release" "pfn-logger" {
  count       = var.enable_pfn_logger ? 1 : 0
  name        = "pfn-logger"
  chart       = local.pfn_logger_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      logger = {
        name = "pfn"
      }
      chain = {
        name = "aptos-${local.workspace_name}"
      }
    }),
    jsonencode(var.pfn_logger_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.pfn_logger_helm_chart_path, "**") : filesha1("${local.pfn_logger_helm_chart_path}/${f}")]))
  }
}

resource "helm_release" "metrics-server" {
  name        = "metrics-server"
  namespace   = "kube-system"
  chart       = "${path.module}/../helm/k8s-metrics"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      coredns = {
        maxReplicas = var.num_validators
      }
      autoscaler = {
        enabled     = true
        clusterName = module.validator.aws_eks_cluster.name
        image = {
          # EKS does not report patch version
          tag = "v${module.validator.aws_eks_cluster.version}.0"
        }
        serviceAccount = {
          annotations = {
            "eks.amazonaws.com/role-arn" = aws_iam_role.cluster-autoscaler.arn
          }
        }
      }
    })
  ]
}


# access control
data "aws_iam_policy_document" "cluster-autoscaler-assume-role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${module.validator.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:sub"
      # the name of the kube-system cluster-autoscaler service account
      values = ["system:serviceaccount:kube-system:cluster-autoscaler"]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "cluster-autoscaler" {
  statement {
    sid = "Autoscaling"
    actions = [
      "autoscaling:SetDesiredCapacity",
      "autoscaling:TerminateInstanceInAutoScalingGroup"
    ]
    resources = ["*"]
    condition {
      test     = "StringEquals"
      variable = "aws:ResourceTag/k8s.io/cluster-autoscaler/${module.validator.aws_eks_cluster.name}"
      values   = ["owned"]
    }
  }

  statement {
    sid = "DescribeAutoscaling"
    actions = [
      "autoscaling:DescribeAutoScalingInstances",
      "autoscaling:DescribeAutoScalingGroups",
      "ec2:DescribeLaunchTemplateVersions",
      "autoscaling:DescribeTags",
      "autoscaling:DescribeLaunchConfigurations"
    ]
    resources = ["*"]
  }
}

resource "aws_iam_role" "cluster-autoscaler" {
  name                 = "aptos-node-testnet-${local.workspace}-cluster-autoscaler"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.cluster-autoscaler-assume-role.json
}

resource "aws_iam_role_policy" "cluster-autoscaler" {
  name   = "Helm"
  role   = aws_iam_role.cluster-autoscaler.name
  policy = data.aws_iam_policy_document.cluster-autoscaler.json
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

  chart       = "${path.module}/../helm/chaos"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      # Only create the ingress if an ACM certificate exists
      ingress = {
        enable                   = length(aws_acm_certificate.ingress) > 0 ? true : false
        domain                   = length(aws_acm_certificate.ingress) > 0 ? "chaos.${local.domain}" : ""
        acm_certificate          = length(aws_acm_certificate.ingress) > 0 ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = join(",", var.client_sources_ipv4)
        aws_tags                 = local.aws_tags
      }
      chaos-mesh = {
        chaosDaemon = {
          podSecurityPolicy = true
          # tolerate pod assignment on nodes in the validator nodegroup
          tolerations = [{
            key    = "aptos.org/nodepool"
            value  = "validators"
            effect = "NoExecute"
          }]
        }
      }
    })
  ]
}

resource "helm_release" "testnet-addons" {
  name        = "testnet-addons"
  chart       = "${path.module}/../helm/testnet-addons"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      aws = {
        region       = var.region
        cluster_name = module.validator.aws_eks_cluster.name
        vpc_id       = module.validator.vpc_id
        role_arn     = aws_iam_role.k8s-aws-integrations.arn
        zone_name    = var.zone_id != "" ? data.aws_route53_zone.aptos[0].name : null
      }
      genesis = {
        era             = var.era
        username_prefix = local.aptos_node_helm_prefix
      }
      service = {
        domain = local.domain
        aws_tags = local.aws_tags
      }
      ingress = {
        acm_certificate          = length(aws_acm_certificate.ingress) > 0 ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = var.client_sources_ipv4
      }
    })
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

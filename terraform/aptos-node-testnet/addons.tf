locals {
  autoscaling_helm_chart_path         = "${path.module}/../helm/autoscaling"
  chaos_mesh_helm_chart_path          = "${path.module}/../helm/chaos"
  testnet_addons_helm_chart_path      = "${path.module}/../helm/testnet-addons"
  node_health_checker_helm_chart_path = "${path.module}/../helm/node-health-checker"
}

resource "helm_release" "autoscaling" {
  name        = "autoscaling"
  namespace   = "kube-system"
  chart       = local.autoscaling_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      coredns = {
        maxReplicas = var.num_validators
      }
      # https://github.com/kubernetes-sigs/metrics-server#scaling
      metrics-server = {
        # 1m core per node
        # 2MiB memory per node
        resources = {
          requests = {
            cpu    = var.validator_instance_max_num > 0 ? "${var.validator_instance_max_num}m" : null,
            memory = var.validator_instance_max_num > 0 ? "${var.validator_instance_max_num * 2}Mi" : null,
          }
        }
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

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.autoscaling_helm_chart_path, "**") : filesha1("${local.autoscaling_helm_chart_path}/${f}")]))
  }
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
  name                 = "aptos-node-testnet-${local.workspace_name}-cluster-autoscaler"
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

  chart       = local.chaos_mesh_helm_chart_path
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

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.chaos_mesh_helm_chart_path, "**") : filesha1("${local.chaos_mesh_helm_chart_path}/${f}")]))
  }
}

// service account used for all external AWS-facing services, such as ALB ingress controller and External-DNS
resource "kubernetes_service_account" "k8s-aws-integrations" {
  metadata {
    name      = "k8s-aws-integrations"
    namespace = "kube-system"
    annotations = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.k8s-aws-integrations.arn
    }
  }
}

# when upgrading the AWS ALB ingress controller, update the CRDs as well using:
# kubectl apply -k "github.com/aws/eks-charts/stable/aws-load-balancer-controller/crds?ref=master"
resource "helm_release" "aws-load-balancer-controller" {
  name        = "aws-load-balancer-controller"
  repository  = "https://aws.github.io/eks-charts"
  chart       = "aws-load-balancer-controller"
  version     = "1.4.3"
  namespace   = "kube-system"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      serviceAccount = {
        create = false
        name   = kubernetes_service_account.k8s-aws-integrations.metadata[0].name
      }
      clusterName = module.validator.aws_eks_cluster.name
      region      = var.region
      vpcId       = module.validator.vpc_id
    })
  ]
}

resource "helm_release" "external-dns" {
  count       = var.zone_id != "" ? 1 : 0
  name        = "external-dns"
  repository  = "https://kubernetes-sigs.github.io/external-dns"
  chart       = "external-dns"
  version     = "1.11.0"
  namespace   = "kube-system"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      serviceAccount = {
        create = false
        name   = kubernetes_service_account.k8s-aws-integrations.metadata[0].name
      }
      domainFilters = var.zone_id != "" ? [data.aws_route53_zone.aptos[0].name] : []
      txtOwnerId    = var.zone_id
    })
  ]
}

resource "helm_release" "testnet-addons" {
  count       = var.enable_forge ? 0 : 1
  name        = "testnet-addons"
  chart       = local.testnet_addons_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      imageTag = var.image_tag
      genesis = {
        era             = var.era
        username_prefix = local.aptos_node_helm_prefix
        chain_id        = var.chain_id
        numValidators   = var.num_validators
      }
      service = {
        domain   = local.domain
        aws_tags = local.aws_tags
      }
      ingress = {
        acm_certificate          = length(aws_acm_certificate.ingress) > 0 ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = var.client_sources_ipv4
      }
      load_test = {
        fullnodeGroups = try(var.aptos_node_helm_values.fullnode.groups, [])
        config = {
          numFullnodeGroups = var.num_fullnode_groups
        }
      }
    }),
    jsonencode(var.testnet_addons_helm_values)
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.testnet_addons_helm_chart_path, "**") : filesha1("${local.testnet_addons_helm_chart_path}/${f}")]))
  }
}

resource "helm_release" "node-health-checker" {
  count       = var.enable_node_health_checker ? 1 : 0
  name        = "node-health-checker"
  chart       = local.node_health_checker_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      imageTag = var.image_tag
      # borrow the serviceaccount for the rest of the testnet addon components
      # TODO: just create a service account for the node-health-checker
      serviceAccount = {
        create = false
        name   = "testnet-addons"
      }
    }),
    jsonencode(var.node_health_checker_helm_values)
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.node_health_checker_helm_chart_path, "**") : filesha1("${local.node_health_checker_helm_chart_path}/${f}")]))
  }
}

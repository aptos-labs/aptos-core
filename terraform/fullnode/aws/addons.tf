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
      clusterName = data.aws_eks_cluster.aptos.name
      region      = var.region
      vpcId       = module.eks.vpc_id
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
      domainFilters = var.zone_id != "" ? [data.aws_route53_zone.pfn[0].name] : []
      txtOwnerId    = var.zone_id
    })
  ]
}

locals {
  autoscaling_helm_chart_path = "${path.module}/../../helm/autoscaling"
}

resource "helm_release" "autoscaling" {
  name        = "autoscaling"
  namespace   = "kube-system"
  chart       = local.autoscaling_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      autoscaler = {
        enabled     = true
        clusterName = data.aws_eks_cluster.aptos.name
        image = {
          # EKS does not report patch version
          tag = "v${data.aws_eks_cluster.aptos.version}.0"
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
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${local.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:sub"
      # the name of the kube-system cluster-autoscaler service account
      values = ["system:serviceaccount:kube-system:cluster-autoscaler"]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:aud"
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
      variable = "aws:ResourceTag/k8s.io/cluster-autoscaler/${data.aws_eks_cluster.aptos.name}"
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
  name                 = "aptos-fullnode-${local.workspace_name}-cluster-autoscaler"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.cluster-autoscaler-assume-role.json
}

resource "aws_iam_role_policy" "cluster-autoscaler" {
  name   = "Helm"
  role   = aws_iam_role.cluster-autoscaler.name
  policy = data.aws_iam_policy_document.cluster-autoscaler.json
}

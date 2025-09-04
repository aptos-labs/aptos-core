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
      clusterName = data.aws_eks_cluster.velor.name
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

resource "helm_release" "pfn-addons" {
  depends_on = [
    helm_release.fullnode
  ]
  name        = "pfn-addons"
  chart       = local.pfn_addons_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      service = {
        domain   = local.domain
        aws_tags = local.aws_tags
        fullnode = {
          numFullnodes             = var.num_fullnodes
          loadBalancerSourceRanges = var.client_sources_ipv4
        }
      }
      ingress = {
        class                    = "alb"
        acm_certificate          = var.zone_id != "" ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = var.client_sources_ipv4
      }
    }),
    jsonencode(var.pfn_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.pfn_addons_helm_chart_path, "**") : filesha1("${local.pfn_addons_helm_chart_path}/${f}")]))
  }
}

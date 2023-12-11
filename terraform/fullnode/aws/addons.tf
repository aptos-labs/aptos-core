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



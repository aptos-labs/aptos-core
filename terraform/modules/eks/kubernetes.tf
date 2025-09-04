provider "kubernetes" {
  host                   = aws_eks_cluster.velor.endpoint
  cluster_ca_certificate = base64decode(aws_eks_cluster.velor.certificate_authority[0].data)
  token                  = data.aws_eks_cluster_auth.velor.token
}

resource "kubernetes_storage_class" "io1" {
  metadata {
    name = "io1"
  }
  storage_provisioner = "kubernetes.io/aws-ebs"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type      = "io1"
    iopsPerGB = "50"
  }
}

resource "kubernetes_storage_class" "gp3" {
  metadata {
    name = "gp3"
    annotations = {
      "storageclass.kubernetes.io/is-default-class" = false
    }
  }
  storage_provisioner = "ebs.csi.aws.com"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "gp3"
  }

  depends_on = [aws_eks_addon.aws-ebs-csi-driver]
}

resource "kubernetes_storage_class" "io2" {
  metadata {
    name = "io2"
  }
  storage_provisioner = "ebs.csi.aws.com"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "io2"
    iops = "40000"
  }
}

resource "null_resource" "delete-gp2" {
  provisioner "local-exec" {
    command = <<-EOT
      aws --region ${var.region} eks update-kubeconfig --name ${aws_eks_cluster.velor.name} --kubeconfig ${local.kubeconfig} &&
      kubectl --kubeconfig ${local.kubeconfig} delete --ignore-not-found storageclass gp2
    EOT
  }

  depends_on = [kubernetes_storage_class.io1]
}


resource "kubernetes_storage_class" "gp2" {
  metadata {
    name = "gp2"
    annotations = {
      "storageclass.kubernetes.io/is-default-class" = true
    }
  }
  storage_provisioner = "kubernetes.io/aws-ebs"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "gp2"
  }

  depends_on = [null_resource.delete-gp2]
}

locals {
  kubeconfig = "/tmp/kube.config.${md5(timestamp())}"
}

provider "helm" {
  kubernetes {
    host                   = aws_eks_cluster.velor.endpoint
    cluster_ca_certificate = base64decode(aws_eks_cluster.velor.certificate_authority[0].data)
    token                  = data.aws_eks_cluster_auth.velor.token
  }
}

resource "kubernetes_cluster_role" "debug" {
  metadata {
    name = "debug"
  }

  rule {
    api_groups = [""]
    resources  = ["pods/portforward", "pods/exec"]
    verbs      = ["create"]
  }
}

resource "kubernetes_role_binding" "debuggers" {
  metadata {
    name = "debuggers"
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = kubernetes_cluster_role.debug.metadata[0].name
  }

  subject {
    kind = "Group"
    name = "debuggers"
  }
}

resource "kubernetes_role_binding" "viewers" {
  metadata {
    name = "viewers"
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = "view"
  }

  subject {
    kind = "Group"
    name = "viewers"
  }
  subject {
    kind = "Group"
    name = "debuggers"
  }
}

resource "kubernetes_config_map" "aws-auth" {
  metadata {
    name      = "aws-auth"
    namespace = "kube-system"
  }

  data = {
    mapRoles = yamlencode(concat(
      [{
        rolearn  = aws_iam_role.nodes.arn
        username = "system:node:{{EC2PrivateDNSName}}"
        groups   = ["system:bootstrappers", "system:nodes"]
      }],
      var.iam_path == "/" ? [] : [{
        # Workaround for https://github.com/kubernetes-sigs/aws-iam-authenticator/issues/268
        # The entry above is still needed otherwise EKS marks the node group as unhealthy
        rolearn  = replace(aws_iam_role.nodes.arn, "role${var.iam_path}", "role/")
        username = "system:node:{{EC2PrivateDNSName}}"
        groups   = ["system:bootstrappers", "system:nodes"]
      }],
      [for role in var.k8s_admin_roles : {
        rolearn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/${role}"
        username = "${role}:{{SessionName}}"
        groups   = ["system:masters"]
      }],
      [for role in var.k8s_viewer_roles : {
        rolearn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/${role}"
        username = "${role}:{{SessionName}}"
        groups   = ["viewers"]
      }],
      [for role in var.k8s_debugger_roles : {
        rolearn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/${role}"
        username = "${role}:{{SessionName}}"
        groups   = ["debuggers"]
      }],
    ))
    mapUsers = yamlencode(concat(
      [for user in var.k8s_admins : {
        userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:user/${user}"
        username = user
        groups   = ["system:masters"]
      }],
      [for user in var.k8s_viewers : {
        userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:user/${user}"
        username = user
        groups   = ["viewers"]
      }],
      [for user in var.k8s_debuggers : {
        userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:user/${user}"
        username = user
        groups   = ["debuggers"]
      }],
    ))
  }
}

resource "local_file" "kubernetes" {
  filename = "${local.workspace_name}-kubernetes.json"
  content = jsonencode({
    kubernetes_host        = aws_eks_cluster.velor.endpoint
    kubernetes_ca_cert     = base64decode(aws_eks_cluster.velor.certificate_authority[0].data)
    issuer                 = aws_eks_cluster.velor.identity[0].oidc[0].issuer
    service_account_prefix = "velor-pfn"
    pod_cidrs              = aws_subnet.private[*].cidr_block
  })
  file_permission = "0644"
}

output "kubernetes" {
  value     = jsondecode(local_file.kubernetes.content)
  sensitive = true
}

output "oidc_provider" {
  value = local.oidc_provider
}

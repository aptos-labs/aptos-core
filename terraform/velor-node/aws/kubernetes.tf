provider "kubernetes" {
  host                   = aws_eks_cluster.velor.endpoint
  cluster_ca_certificate = base64decode(aws_eks_cluster.velor.certificate_authority[0].data)
  token                  = data.aws_eks_cluster_auth.velor.token
}

provider "helm" {
  kubernetes {
    host                   = aws_eks_cluster.velor.endpoint
    cluster_ca_certificate = base64decode(aws_eks_cluster.velor.certificate_authority[0].data)
    token                  = data.aws_eks_cluster_auth.velor.token
  }
}

locals {
  kubeconfig = "/tmp/kube.config.${md5(timestamp())}"

  # helm chart paths
  velor_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/velor-node"
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"
}

resource "null_resource" "delete-gp2" {
  provisioner "local-exec" {
    command = <<-EOT
      aws --region ${var.region} eks update-kubeconfig --name ${aws_eks_cluster.velor.name} --kubeconfig ${local.kubeconfig} &&
      kubectl --kubeconfig ${local.kubeconfig} delete --ignore-not-found storageclass gp2
    EOT
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

  depends_on = [null_resource.delete-gp2]
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

locals {
  helm_values = jsonencode({
    numValidators     = var.num_validators
    numFullnodeGroups = var.num_fullnode_groups
    imageTag          = var.image_tag
    manageImages      = var.manage_via_tf # if we're managing the entire deployment via terraform, override the images as well
    chain = {
      era      = var.era
      chain_id = var.chain_id
      name     = var.chain_name
    }
    validator = {
      name = var.validator_name
      storage = {
        class = var.validator_storage_class
      }
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "validators"
      }
      tolerations = [{
        key    = "velor.org/nodepool"
        value  = "validators"
        effect = "NoExecute"
      }]
    }
    fullnode = {
      storage = {
        class = var.fullnode_storage_class
      }
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "validators"
      }
      tolerations = [{
        key    = "velor.org/nodepool"
        value  = "validators"
        effect = "NoExecute"
      }]
    }
    haproxy = {
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "utilities"
      }
    }
    service = {
      domain = local.domain
    }
  })

  # override the helm release name if an override exists, otherwise adopt the workspace name
  helm_release_name = var.helm_release_name_override != "" ? var.helm_release_name_override : local.workspace_name
}

resource "helm_release" "validator" {
  count       = var.helm_enable_validator ? 1 : 0
  name        = local.helm_release_name
  chart       = local.velor_node_helm_chart_path
  max_history = 5
  wait        = false

  # lifecycle {
  #   ignore_changes = [
  #     values,
  #   ]
  # }

  values = [
    local.helm_values,
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.velor_node_helm_chart_path, "**") : filesha1("${local.velor_node_helm_chart_path}/${f}")]))
    }
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

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "${local.helm_release_name}-mon"
  chart       = local.monitoring_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      chain = {
        name = var.chain_name
      }
      validator = {
        name = var.validator_name
      }
      service = {
        domain = local.domain
      }
      monitoring = {
        prometheus = {
          storage = {
            class = kubernetes_storage_class.gp3.metadata[0].name
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

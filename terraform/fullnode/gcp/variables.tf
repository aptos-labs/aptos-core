variable "project" {
  description = "GCP project"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
}

variable "zone" {
  description = "GCP zone suffix"
  type        = string
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "tls_sans" {
  description = "List of Subject Alternate Names to include in TLS certificate"
  type        = list(string)
  default     = []
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  default     = true
}

variable "dns_prefix_name" {
  description = "DNS prefix for fullnode url"
  default     = "fullnode"
}

variable "zone_name" {
  description = "Zone name of GCP Cloud DNS zone to create records in"
  default     = ""
}

variable "zone_project" {
  description = "GCP project which the DNS zone is in (if different)"
  default     = ""
}

variable "create_google_managed_ssl_certificate" {
  description = "Whether to create a Google Managed SSL Certificate for the GCE Ingress"
  default     = false
}

variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "pfn_helm_values" {
  description = "Map of values to pass to pfn-addons Helm"
  type        = any
  default     = {}
}

variable "fullnode_helm_values" {
  description = "Map of values to pass to public fullnode Helm"
  type        = any
  default     = {}
}

variable "fullnode_helm_values_list" {
  description = "List of values to pass to public fullnode, for setting different value per node. length(fullnode_helm_values_list) must equal var.num_fullnodes"
  type        = any
  default     = {}
}

variable "k8s_namespace" {
  default     = "aptos"
  description = "Kubernetes namespace that the fullnode will be deployed into"
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "num_fullnodes" {
  default     = 1
  description = "Number of fullnodes"
}

variable "num_extra_instance" {
  default     = 0
  description = "Number of extra instances to add into node pool"
}

variable "instance_disk_size_gb" {
  default     = 100
  description = "Disk size for fullnode instance"
}

variable "image_tag" {
  default     = "devnet"
  description = "Docker image tag to use for the fullnode"
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 1
}

variable "chain_id" {
  description = "aptos chain ID"
  default     = "DEVNET"
}

variable "chain_name" {
  description = "Aptos chain name"
  default     = "devnet"
}

variable "fullnode_name" {
  description = "Name of the fullnode node owner"
  type        = string
}

variable "machine_type" {
  description = "Machine type for running fullnode"
  default     = "n2-standard-32"
}

variable "enable_backup" {
  description = "enable data backup from fullnode"
  default     = false
}

variable "enable_public_backup" {
  description = "provide data backups to the public"
  default     = false
}


variable "backup_fullnode_index" {
  description = "index of fullnode to backup data from"
  default     = 0
}

variable "enable_monitoring" {
  description = "Enable monitoring helm chart"
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring Helm"
  type        = any
  default     = {}
}

variable "enable_prometheus_node_exporter" {
  description = "Enable prometheus-node-exporter within monitoring helm chart"
  default     = false
}

variable "enable_kube_state_metrics" {
  description = "Enable kube-state-metrics within monitoring helm chart"
  default     = false
}

variable "gke_enable_private_nodes" {
  description = "Enable private nodes for GKE cluster"
  default     = true
}

variable "gke_enable_node_autoprovisioning" {
  description = "Enable node autoprovisioning for GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/how-to/node-auto-provisioning"
  default     = false
}

variable "gke_node_autoprovisioning_max_cpu" {
  description = "Maximum CPU utilization for GKE node_autoprovisioning"
  default     = 10
}

variable "gke_node_autoprovisioning_max_memory" {
  description = "Maximum memory utilization for GKE node_autoprovisioning"
  default     = 100
}

variable "gke_enable_autoscaling" {
  description = "Enable autoscaling for the nodepools in the GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-autoscaler"
  default     = true
}

variable "gke_autoscaling_max_node_count" {
  description = "Maximum number of nodes for GKE nodepool autoscaling"
  default     = 10
}

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  default     = true
}

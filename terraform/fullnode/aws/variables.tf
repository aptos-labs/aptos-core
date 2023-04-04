variable "region" {
  description = "AWS region"
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "iam_path" {
  default     = "/"
  description = "Path to use when naming IAM objects"
}

variable "permissions_boundary_policy" {
  default     = ""
  description = "ARN of IAM policy to set as permissions boundary on created roles"
}

variable "admin_sources_ipv4" {
  description = "List of CIDR subnets which can access Kubernetes API"
  type        = list(string)
}

variable "client_sources_ipv4" {
  description = "List of CIDR subnets which can access the testnet API"
  type        = list(string)
}

variable "k8s_admin_roles" {
  description = "List of AWS roles to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_admins" {
  description = "List of AWS usernames to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "num_fullnodes" {
  default = 1
}

variable "image_tag" {
  description = "Docker image tag for aptos components. Overrides ecr_repo method."
  default     = ""
}

variable "ecr_repo" {
  description = "Name of an ECR repo to resolve 'stable' tag to a specific revision"
  default     = ""
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 15
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

variable "zone_id" {
  description = "Route53 Zone ID to create records in"
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

variable "enable_pfn_logger" {
  description = "Enable separate public fullnode logger pod"
  default     = false
}

variable "pfn_logger_helm_values" {
  description = "Map of values to pass to public fullnode logger Helm"
  type        = any
  default     = {}
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.medium"
}

variable "fullnode_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c6i.8xlarge"
}

variable "num_extra_instance" {
  default     = 0
  description = "Number of extra instances to add into node pool"
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

variable "fullnode_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  default     = "io1"
  validation {
    condition     = contains(["gp3", "gp2", "io1", "io2"], var.fullnode_storage_class)
    error_message = "Supported storage classes are gp3, io1, io2"
  }
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

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  default     = true
}

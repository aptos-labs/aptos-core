variable "region" {
  description = "AWS region"
  type        = string
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

variable "iam_path" {
  description = "Path to use when naming IAM objects"
  type        = string
  default     = "/"
}

variable "permissions_boundary_policy" {
  description = "ARN of IAM policy to set as permissions boundary on created roles"
  type        = string
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
  description = "Number of fullnodes."
  type        = number
  default     = 1
}

variable "image_tag" {
  description = "Docker image tag for velor components. Overrides ecr_repo method."
  type        = string
  default     = ""
}

variable "ecr_repo" {
  description = "Name of an ECR repo to resolve 'stable' tag to a specific revision"
  type        = string
  default     = ""
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 15
}

variable "chain_id" {
  description = "Velor chain ID"
  type        = string
  default     = "DEVNET"
}

variable "chain_name" {
  description = "Velor chain name"
  type        = string
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
  type        = string
  default     = ""
}

variable "tls_sans" {
  description = "List of Subject Alternate Names to include in TLS certificate"
  type        = list(string)
  default     = []
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  type        = bool
  default     = true
}

variable "dns_prefix_name" {
  description = "DNS prefix for fullnode url"
  type        = string
  default     = "fullnode"
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "t3.medium"
}

variable "fullnode_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "c6i.16xlarge"
}

variable "num_extra_instance" {
  description = "Number of extra instances to add into node pool"
  type        = number
  default     = 0
}

variable "enable_backup" {
  description = "Enable data backup from fullnode"
  type        = bool
  default     = false
}

variable "enable_public_backup" {
  description = "Provide data backups to the public"
  type        = bool
  default     = false
}

variable "backup_fullnode_index" {
  description = "Index of fullnode to backup data from"
  type        = number
  default     = 0
}

variable "fullnode_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  type        = string
  default     = "io1"
  validation {
    condition     = contains(["gp2", "gp3", "io1", "io2"], var.fullnode_storage_class)
    error_message = "Supported storage classes are gp2, gp3, io1, io2"
  }
}

variable "fullnode_storage_size" {
  description = "Disk size for fullnodes"
  type        = string
  default     = "2000Gi"
}

variable "enable_monitoring" {
  description = "Enable monitoring helm chart"
  type        = bool
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring Helm"
  type        = any
  default     = {}
}

variable "enable_prometheus_node_exporter" {
  description = "Enable prometheus-node-exporter within monitoring helm chart"
  type        = bool
  default     = false
}

variable "enable_kube_state_metrics" {
  description = "Enable kube-state-metrics within monitoring helm chart"
  type        = bool
  default     = false
}

variable "manage_via_tf" {
  description = "Whether to manage the velor-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  type        = bool
  default     = true
}

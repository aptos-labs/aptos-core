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

variable "pfn_helm_values" {
  description = "Map of values to pass to testnet Helm"
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
  default     = "c6i.4xlarge"
}

variable "num_extra_instance" {
  default     = 0
  description = "Number of extra instances to add into node pool"
}

variable "enable_backup" {
  description = "enable data backup from fullnode"
  default     = false
}

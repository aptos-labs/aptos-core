### Infrastructure config 

variable "region" {
  description = "AWS region"
}

variable "zone_id" {
  description = "Route53 Zone ID to create records in"
  default     = ""
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
  default     = ["0.0.0.0/0"]
}

variable "client_sources_ipv4" {
  description = "List of CIDR subnets which can access the testnet API"
  type        = list(string)
  default     = ["0.0.0.0/0"]
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

### Testnet config

variable "chain_id" {
  description = "Aptos chain ID"
  default     = 4
}


variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 15
}

variable "chain_name" {
  description = "Aptos chain name"
  default     = "devnet"
}

variable "image_tag" {
  description = "Docker image tag for Aptos node"
  default     = "devnet"
}

### Helm values

variable "aptos_node_helm_values" {
  description = "Map of values to pass to aptos-node helm chart"
  type        = any
  default     = {}
}

variable "genesis_helm_values" {
  description = "Map of values to pass to genesis helm chart"
  type        = any
  default     = {}
}

variable "logger_helm_values" {
  description = "Map of values to pass to logger helm chart"
  type        = any
  default     = {}
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring helm chart"
  type        = any
  default     = {}
}


### EKS nodegroups

variable "num_validators" {
  default = 4
}

variable "num_utility_instance" {
  description = "Number of instances for utilities node pool, when it's 0, it will be set to var.num_validators"
  default     = 0
}

variable "num_validator_instance" {
  description = "Number of instances for validator node pool, when it's 0, it will be set to 2 * var.num_validators"
  default     = 0
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.medium"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c5.xlarge"
}


### Infrastructure config 

variable "region" {
  description = "AWS region"
  type        = string
}

variable "maximize_single_az_capacity" {
  description = "TEST ONLY: Whether to maximize the capacity of the cluster by allocating a large CIDR block to the first AZ"
  type        = bool
  default     = false
}

variable "zone_id" {
  description = "Route53 Zone ID to create records in"
  type        = string
  default     = ""
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
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
  description = "Velor chain ID. If var.enable_forge set, defaults to 4"
  type        = number
  default     = 4
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 15
}

variable "chain_name" {
  description = "Velor chain name. If unset, defaults to using the workspace name"
  type        = string
  default     = ""
}

variable "image_tag" {
  description = "Docker image tag for all Velor workloads, including validators, fullnodes, backup, restore, genesis, and other tooling"
  type        = string
  default     = "devnet"
}

variable "validator_image_tag" {
  description = "Docker image tag for validators and fullnodes. If set, overrides var.image_tag for those nodes"
  type        = string
  default     = ""
}

### Helm values

variable "velor_node_helm_values" {
  description = "Map of values to pass to velor-node helm chart"
  type        = any
  default     = {}
}

variable "genesis_helm_values" {
  description = "Map of values to pass to genesis helm chart"
  type        = any
  default     = {}
}

variable "enable_genesis" {
  description = "Perform genesis automatically"
  type        = bool
  default     = true
}

variable "testnet_addons_helm_values" {
  description = "Map of values to pass to testnet-addons helm chart"
  type        = any
  default     = {}
}

### EKS nodegroups

variable "num_validators" {
  description = "The number of validator nodes to create"
  type        = number
  default     = 4
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  type        = number
  default     = 1
}

variable "num_utility_instance" {
  description = "Number of instances for utilities node pool, when it's 0, it will be set to var.num_validators"
  type        = number
  default     = 0
}

variable "num_validator_instance" {
  description = "Number of instances for validator node pool, when it's 0, it will be set to 2 * var.num_validators"
  type        = number
  default     = 0
}

variable "utility_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.num_validators"
  type        = number
  default     = 0
}

variable "validator_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.num_validators"
  type        = number
  default     = 0
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "t3.2xlarge"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "c6i.16xlarge"
}

### Forge

variable "enable_forge" {
  description = "Enable Forge test framework, also creating an internal helm repo"
  type        = bool
  default     = false
}

variable "forge_config_s3_bucket" {
  description = "S3 bucket in which Forge config is stored"
  type        = string
  default     = "forge-wrapper-config"
}

variable "forge_helm_values" {
  description = "Map of values to pass to Forge Helm"
  type        = any
  default     = {}
}

variable "validator_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  type        = string
  default     = "io1"
  validation {
    condition     = contains(["gp3", "io1", "io2"], var.validator_storage_class)
    error_message = "Supported storage classes are gp3, io1, io2"
  }
}

variable "fullnode_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  type        = string
  default     = "io1"
  validation {
    condition     = contains(["gp3", "io1", "io2"], var.fullnode_storage_class)
    error_message = "Supported storage classes are gp3, io1, io2"
  }
}

variable "manage_via_tf" {
  description = "Whether to manage the velor-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  type        = bool
  default     = true
}

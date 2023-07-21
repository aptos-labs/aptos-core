### Infrastructure config 

variable "region" {
  description = "AWS region"
}

variable "maximize_single_az_capacity" {
  description = "TEST ONLY: Whether to maximize the capacity of the cluster by allocating a large CIDR block to the first AZ"
  default     = false
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
  description = "Aptos chain ID. If var.enable_forge set, defaults to 4"
  default     = 4
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 15
}

variable "chain_name" {
  description = "Aptos chain name. If unset, defaults to using the workspace name"
  default     = ""
}

variable "image_tag" {
  description = "Docker image tag for all Aptos workloads, including validators, fullnodes, backup, restore, genesis, and other tooling"
  default     = "devnet"
}

variable "validator_image_tag" {
  description = "Docker image tag for validators and fullnodes. If set, overrides var.image_tag for those nodes"
  default     = ""
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

variable "enable_monitoring" {
  description = "Enable monitoring helm chart"
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring helm chart"
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

variable "testnet_addons_helm_values" {
  description = "Map of values to pass to testnet-addons helm chart"
  type        = any
  default     = {}
}

variable "enable_node_health_checker" {
  description = "Enable node-health-checker"
  default     = false
}

variable "node_health_checker_helm_values" {
  description = "Map of values to pass to node-health-checker helm chart"
  type        = any
  default     = {}
}

### EKS nodegroups

variable "num_validators" {
  description = "The number of validator nodes to create"
  default     = 4
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  default     = 1
}

variable "num_utility_instance" {
  description = "Number of instances for utilities node pool, when it's 0, it will be set to var.num_validators"
  default     = 0
}

variable "num_validator_instance" {
  description = "Number of instances for validator node pool, when it's 0, it will be set to 2 * var.num_validators"
  default     = 0
}

variable "utility_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.num_validators"
  default     = 0
}

variable "validator_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.num_validators"
  default     = 0
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.2xlarge"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c6i.4xlarge"
}

### Forge

variable "enable_forge" {
  description = "Enable Forge test framework, also creating an internal helm repo"
  default     = false
}

variable "forge_config_s3_bucket" {
  description = "S3 bucket in which Forge config is stored"
  default     = "forge-wrapper-config"
}

variable "forge_helm_values" {
  description = "Map of values to pass to Forge Helm"
  type        = any
  default     = {}
}

variable "validator_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  default     = "io1"
  validation {
    condition     = contains(["gp3", "io1", "io2"], var.validator_storage_class)
    error_message = "Supported storage classes are gp3, io1, io2"
  }
}

variable "fullnode_storage_class" {
  description = "Which storage class to use for the validator and fullnode"
  default     = "io1"
  validation {
    condition     = contains(["gp3", "io1", "io2"], var.fullnode_storage_class)
    error_message = "Supported storage classes are gp3, io1, io2"
  }
}

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  default     = true
}

variable "region" {
  description = "AWS region"
  type        = string
}

variable "num_azs" {
  description = "Number of availability zones"
  type        = number
  default     = 3
}

variable "kubernetes_version" {
  description = "Version of Kubernetes to use for EKS cluster"
  type        = string
  default     = "1.26"
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "num_validators" {
  description = "The number of validator nodes to create"
  type        = number
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  type        = number
  default     = 1
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 1
}

variable "chain_id" {
  description = "Velor chain ID"
  type        = string
  default     = "TESTING"
}

variable "chain_name" {
  description = "Velor chain name"
  type        = string
  default     = "testnet"
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag for Velor node"
  type        = string
  default     = "devnet"
}

variable "zone_id" {
  description = "Zone ID of Route 53 domain to create records in"
  type        = string
  default     = ""
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  type        = bool
  default     = true
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  type        = string
  default     = "<workspace>.velor"
}

variable "create_records" {
  description = "Creates DNS records in var.zone_id that point to k8s service, as opposed to using external-dns or other means"
  type        = bool
  default     = true
}

variable "helm_chart" {
  description = "Path to velor-validator Helm chart file"
  type        = string
  default     = ""
}

variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "helm_values_file" {
  description = "Path to file containing values for Helm chart"
  type        = string
  default     = ""
}

variable "k8s_admins" {
  description = "List of AWS usernames to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_admin_roles" {
  description = "List of AWS roles to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_viewers" {
  description = "List of AWS usernames to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_viewer_roles" {
  description = "List of AWS roles to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_debuggers" {
  description = "List of AWS usernames to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "k8s_debugger_roles" {
  description = "List of AWS roles to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
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

variable "vpc_cidr_block" {
  description = "VPC CIDR Block"
  type        = string
  default     = "192.168.0.0/16"
}

variable "maximize_single_az_capacity" {
  description = "Whether to maximize the capacity of the cluster by allocating more IPs to the first AZ"
  type        = bool
  default     = false
}

variable "helm_enable_validator" {
  description = "Enable deployment of the validator Helm chart"
  type        = bool
  default     = true
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "t3.2xlarge"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  type        = number
  default     = 1
}

variable "utility_instance_min_num" {
  description = "Minimum number of instances for utilities"
  type        = number
  default     = 1
}

variable "utility_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.utility_instance_num"
  type        = number
  default     = 0
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint the instances in the utility nodegroup"
  type        = bool
  default     = false
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "c6i.16xlarge"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  type        = number
  default     = 2
}

variable "validator_instance_min_num" {
  description = "Minimum number of instances for validators"
  type        = number
  default     = 1
}

variable "validator_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.validator_instance_num"
  type        = number
  default     = 0
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  type        = bool
  default     = false
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

variable "helm_release_name_override" {
  description = "If set, overrides the name of the velor-node helm chart"
  type        = string
  default     = ""
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

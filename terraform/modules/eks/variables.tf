variable "region" {
  description = "AWS region"
  type        = string
}

variable "kubernetes_version" {
  description = "Version of Kubernetes to use for EKS cluster"
  type        = string
  default     = "1.28"
}

variable "eks_cluster_name" {
  description = "Name of the eks cluster"
  type        = string
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
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

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "t3.medium"
}

variable "fullnode_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "c6i.8xlarge"
}

variable "num_fullnodes" {
  description = "Number of fullnodes to deploy"
  type        = number
  default     = 1
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

variable "num_extra_instance" {
  description = "Number of extra instances to add into node pool"
  type        = number
  default     = 0
}

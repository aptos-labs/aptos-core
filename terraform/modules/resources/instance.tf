### Inputs

variable "instance_type" {
  description = "The instance type"
  type        = string
  default     = ""

  validation {
    condition     = can(regex("^(c2|e2|n2d|t2d)-standard-(4|8|16|32|48|60)$", var.instance_type))
    error_message = "Unknown machine type"
  }
}

variable "utility_instance_type" {
  description = "The utilities instance type"
  type        = string
  default     = "e2-standard-8"

  validation {
    condition     = can(regex("^(c2|e2|n2d|t2d)-standard-(4|8|16|32|48|60)$", var.utility_instance_type))
    error_message = "Unknown machine type"
  }
}

variable "max_instances" {
  description = "The maximum number of instances"
  type        = number
  default     = 100
}

variable "app_service" {
  description = "Application service labeled using app.kubernetes.io/part-of"
  type        = string
  default     = ""
}

### Computation

locals {
  machine_family         = split("-", var.instance_type)[0]
  utility_machine_family = split("-", var.utility_instance_type)[0]
  machine_shapes = {
    "c2-standard-60"  = { cores = 60, memory = 240 }
    "t2d-standard-8"  = { cores = 8, memory = 32 }
    "t2d-standard-16" = { cores = 16, memory = 64 }
    "t2d-standard-32" = { cores = 32, memory = 128 }
    "t2d-standard-48" = { cores = 48, memory = 192 }
    "t2d-standard-60" = { cores = 60, memory = 240 }
  }
  # leave 2 cores for the system
  available_cores = local.machine_shapes[var.instance_type].cores - 2
  # leave 4 GB for the system
  available_memory = local.machine_shapes[var.instance_type].memory - 4

  node_affinity = {
    podAntiAffinity = { # don't schedule nodes on the same host
      requiredDuringSchedulingIgnoredDuringExecution = [
        {
          labelSelector = {
            matchExpressions = [
              {
                key      = "app.kubernetes.io/part-of",
                operator = "In",
                values   = [var.app_service]
              }
            ]
          }
          topologyKey = "kubernetes.io/hostname"
        }
      ]
    }
    nodeAffinity = { # affinity for the right instance types
      requiredDuringSchedulingIgnoredDuringExecution = {
        nodeSelectorTerms = [
          {
            matchExpressions = [
              {
                key      = "cloud.google.com/machine-family",
                operator = "In",
                values   = [local.machine_family],
              }
            ]
          }
        ]
      }
    }
  }

  utility_affinity = {
    podAntiAffinity = { # don't schedule nodes on the same host
      requiredDuringSchedulingIgnoredDuringExecution = [
        {
          labelSelector = {
            matchExpressions = [
              {
                key      = "app.kubernetes.io/part-of",
                operator = "In",
                values   = [var.app_service]
              }
            ]
          }
          topologyKey = "kubernetes.io/hostname"
        }
      ]
    }
    nodeAffinity = { # affinity for the right instance types
      requiredDuringSchedulingIgnoredDuringExecution = {
        nodeSelectorTerms = [
          {
            matchExpressions = [
              {
                key      = "cloud.google.com/machine-family",
                operator = "In",
                values   = [local.utility_machine_family],
              }
            ]
          }
        ]
      }
    }
  }
}

### Outputs

output "resources" {
  description = "Resources for the instance"
  value = {
    limits = {
      cpu               = local.available_cores
      memory            = "${local.available_memory}G"
      ephemeral-storage = "5Gi"
    }
    requests = {
      cpu               = local.available_cores
      memory            = "${local.available_memory}G"
      ephemeral-storage = "5Gi"
    }
  }
}

output "max_cpu" {
  description = "Maximum CPU for the Node autoprovisioning"
  value       = local.machine_shapes[var.instance_type].cores * var.max_instances
}

output "max_memory" {
  description = "Maximum RAM for the Node autoprovisioning"
  value       = local.machine_shapes[var.instance_type].memory * var.max_instances
}

output "node_affinity" {
  description = "Node affinity for the validator instances"
  value       = local.node_affinity
}

output "utility_affinity" {
  description = "Node affinity for the utility instances"
  value       = local.utility_affinity
}

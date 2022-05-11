terraform {
  required_providers {
    vultr = {
      source  = "vultr/vultr"
      version = "2.10.1"
    }
  }
}

provider "vultr" {
  api_key     = var.api_key
  rate_limit  = 700
  retry_limit = 3
}

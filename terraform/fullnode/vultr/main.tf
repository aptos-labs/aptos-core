terraform {
  required_providers {
    vultr = {
      source = "vultr/vultr"
      version = "2.10.1"
    }
  }
}

provider "local" {}

provider "vultr" {
  api_key = "ZOFSIC7KLHEBTGVA37SZ7KNSNIJRVL4HJXWA"
  rate_limit = 700
  retry_limit = 3
}
packer {
  required_plugins {
    amazon = {
      version = ">= 1.2.8"
      source  = "github.com/hashicorp/amazon"
    }
  }
}

variable "region" {
  type = string
}

variable "ami_prefix" {
  type = string
}

# RunsOn base image; ubuntu24-minimal ships only the runner and Docker, so the build adds what CI jobs use.
variable "source_image" {
  type    = string
  default = "ubuntu22-full-x64"
}

data "amazon-ami" "runs-on-ami-x64" {
  filters = {
    name                = "runs-on-v2.2-${var.source_image}-*"
    root-device-type    = "ebs"
    virtualization-type = "hvm"
  }
  most_recent = true
  # The Runs-On AMI is in the following account
  # ref: https://runs-on.com/guides/building-custom-ami-with-packer/
  owners      = ["135269210855"]
  region      = "${var.region}"
}

source "amazon-ebs" "build-ebs" {
  ami_name       = "${var.ami_prefix}-runs-on-${formatdate("YYYY-MM-DD-hhmmss", timestamp())}"
  instance_type  = "c7a.4xlarge"
  region         = "${var.region}"
  source_ami     = "${data.amazon-ami.runs-on-ami-x64.id}"
  ssh_username   = "ubuntu"
  user_data_file = "./user_data.sh"
}

build {
  name = "setup-runs-on-runner"
  sources = [
    "source.amazon-ebs.build-ebs"
  ]

  provisioner "file" {
    source      = "../../rust-toolchain.toml"
    destination = "/tmp/rust-toolchain.toml"
  }

  provisioner "file" {
    source      = "../../scripts"
    destination = "/tmp/scripts"
  }

  provisioner "file" {
    source      = "./install_runner_tools.sh"
    destination = "/tmp/install_runner_tools.sh"
  }

  provisioner "shell" {
    inline = [
      "whoami",
      "sudo bash /tmp/install_runner_tools.sh",
      "chmod +x /tmp/scripts/dev_setup.sh",
      "sudo -u runner /tmp/scripts/dev_setup.sh -b -r -y -P -J -t -k",
    ]
  }
}

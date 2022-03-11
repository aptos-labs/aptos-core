Aptos Testnet Deployment
========================

This directory contains Terraform configs to deploy a self-contained Aptos
testnet with multiple validators. It uses the [Helm chart][],
[AWS Terraform][] and [vault-init Terraform][] to create an environment which
is very similar to production and able to test most of the production configs.

This README contains documentation on how to connect to an existing testnet and
also how to create an entirely new one. 

Note: the testnet Terraform accepts various variables to configure testnet
resources. These will need to be provided on each invocation of
`terraform apply/plan/destroy`. In the following examples, we will use the
placeholder: `-var-file testnet.tfvars`. A similar placeholder will
be used for the Terraform backend configuration.


Using an existing deployment
---------------------------

1. Install dependencies:
   * Terraform 1.0.0: https://www.terraform.io/downloads.html
   * kubectl: https://kubernetes.io/docs/tasks/tools/install-kubectl/

2. Setup cloud access if necessary.

3. Initialize Terraform, providing your S3 backend details (see [example backend.tfvars file](backend.tfvars.example)):

       $ terraform init -backend-config backend.tfvars

4. Switch to the existing workspace:

       $ terraform workspace select dev

5. Apply the Terraform, targeting the Kubernetes cluster and Vault server (see [example testnet.tfvars file](testnet.tfvars.example)):

       $ terraform apply -var-file testnet.tfvars -target module.validator

6. Configure `kubectl` with the Kubernetes cluster:

       $ aws eks update-kubeconfig --name aptos-<workspace>

Creating a new deployment
-------------------------

1. Install dependencies:
   * Terraform 1.1.0: https://www.terraform.io/downloads.html
   * kubectl: https://kubernetes.io/docs/tasks/tools/install-kubectl/
   * Vault: https://www.vaultproject.io/downloads

2. Setup cloud access if necessary.

3. Initialize Terraform, providing your S3 backend details ([example backend.tfvars file](backend.tfvars.example)):

       $ terraform init -backend-config backend.tfvars

4. If using an existing workspace, switch to it:

       $ terraform workspace select dev

   Or create a new workspace for a new deployment:

       $ terraform workspace new $USER

5. Apply the Terraform, targeting the Kubernetes cluster and Vault server (see [example testnet.tfvars file](testnet.tfvars.example)):

       $ terraform apply -var-file testnet.tfvars -target module.validator

6. Apply the Terraform, targeting the Vault initialization:

       $ terraform apply -var-file testnet.tfvars -target null_resource.vault-init

7. Apply the entire configuration, providing your variable customizations:

       $ terraform apply -var-file testnet.tfvars

8. Configure `kubectl` with the Kubernetes cluster:

       $ aws eks update-kubeconfig --name aptos-<workspace>

[Helm chart]: ../helm/validator
[AWS Terraform]: ../validator/aws/
[vault-init Terraform]: ../validator/vault-init/

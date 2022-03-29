Aptos Fullnodes GCP Deployment
==============================

This directory contains Terraform configs to deploy a public fullnodes on Google Cloud.

1. Install pre-requisites if needed:

   * Terraform 1.1.7: https://www.terraform.io/downloads.html
   * Docker: https://www.docker.com/products/docker-desktop
   * Kubernetes cli: https://kubernetes.io/docs/tasks/tools/
   * Google Cloud cli: https://cloud.google.com/sdk/docs/install-sdk

2. Create a directory for your configuration:

   * Choose a workspace name e.g. `devnet`. Note: this defines terraform workspace name, which in turn is used to form resource names.

         $ export WORKSPACE=devnet

   * Create a directory for the workspace

         $ mkdir -p ~/$WORKSPACE

3. Create a storage bucket for storing the Terraform state on Google Cloud Storage.

4. Copy `backend.tfvars` to `~/$WORKSPACE/backend.tfvars` and edit to fill in your storage bucket name. For more detail on remote state see the Terraform documentation: https://www.terraform.io/docs/backends/index.html

       $ cp backend.tfvars ~/$WORKSPACE/backend.tfvars
       $ vi ~/$WORKSPACE/backend.tfvars

5. Initialise Terraform, providing your backend storage configuration:

       $ terraform init -backend-config ~/$WORKSPACE/backend.tfvars

6. Create a new Terraform workspace to isolate your environments:

        $ terraform workspace new $WORKSPACE

7. Copy `terraform.tfvars` to `~/$WORKSPACE/terraform.tfvars` and edit to set your region and project name:

       $ cp terraform.tfvars ~/$WORKSPACE/terraform.tfvars
       $ vi ~/$WORKSPACE/terraform.tfvars

8. Apply the configuration.

       $ terraform apply -var-file ~/$WORKSPACE/terraform.tfvars

9. Configure your Kubernetes client:

        $ gcloud container clusters get-credentials aptos-$WORKSPACE --zone <region_zone_name> --project <project_name>
        # for example:
        $ gcloud container clusters get-credentials aptos-$WORKSPACE --zone us-central1-a --project aptos-fullnode

10. Check that your fullnode pods are now running (this may take a few minutes):

        $ kubectl get pods

11. Get your fullnode IP:

        $ kubectl get svc -o custom-columns=IP:status.loadBalancer.ingress

12. Check REST API, make sure the ledge version is increasing.

        $ curl http://<IP>
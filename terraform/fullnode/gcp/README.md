Velor Fullnodes GCP Deployment
==============================

This directory contains Terraform configs to deploy a public fullnode on Google Cloud.

These instructions assume that you have a functioning GCP project.  If you do not, see these instructions to create a project.  Run these commands from the cloud shell or from a VM in your GCP project.  


1. Install pre-requisites if needed:

   * Terraform 1.1.7: https://www.terraform.io/downloads.html
   * Docker: https://www.docker.com/products/docker-desktop
   * Kubernetes cli: https://kubernetes.io/docs/tasks/tools/
   * Google Cloud cli: https://cloud.google.com/sdk/docs/install-sdk

   Once you have installed the gcloud CLI, log into GCP using gcloud (https://cloud.google.com/sdk/gcloud/reference/auth/login)

         $ gcloud auth login --update-adc


2. Clone the velor-core repo and go to the terraform gcp folder.

         $ git clone https://github.com/velor-chain/velor-core.git

         $ cd velor-core/terraform/fullnode/gcp

3. Create a working directory for your configuration.  Copy the files you will change so it does not interfere with the cloned repo:

   * Choose a workspace name e.g. `devnet`. Note: this defines terraform workspace name, which in turn is used to form resource names.

         $ export WORKSPACE=devnet

   * Create a directory for the workspace

         $ mkdir -p ~/$WORKSPACE

4. Create a storage bucket for storing the Terraform state on Google Cloud Storage.  Use the console or this gcs command to create the bucket.  See the Google Cloud Storage documentation here: https://cloud.google.com/storage/docs/creating-buckets#prereq-cli

         $ gsutil mb gs://BUCKET_NAME

5. Copy `backend.tfvars` to `~/$WORKSPACE/backend.tfvars` and edit to fill in your storage bucket name. For more detail on remote state see the Terraform documentation: https://www.terraform.io/docs/backends/index.html

       $ cp backend.tfvars ~/$WORKSPACE/backend.tfvars
       $ vi ~/$WORKSPACE/backend.tfvars

6. Initialise Terraform, providing your backend storage configuration.  The storage bucket will keep the 'state' of the terraform operations:

       $ terraform init -backend-config ~/$WORKSPACE/backend.tfvars

7. Create a new Terraform workspace to isolate your environments:

        $ terraform workspace new $WORKSPACE

8. Copy `terraform.tfvars` to `~/$WORKSPACE/terraform.tfvars` and edit to set your region, zone and project name.  If you are having trouble connecting to the devnet and need to add upstream seed peers, uncomment the "fullnode_helm_values" JSON stanza.  For more detail on upstream seed peers, see the documention: https://velor.dev/tutorials/run-a-fullnode/#add-upstream-seed-peers

       $ cp terraform.tfvars ~/$WORKSPACE/terraform.tfvars
       $ vi ~/$WORKSPACE/terraform.tfvars

9. Apply the configuration.  Note that you should be in the velor-core/terraform/fullnode/gcp folder when you run this command.  It will use the  config files that you modified in the ~/$WORKSPACE folder plus the cloned terraform files.

       $ terraform apply -var-file ~/$WORKSPACE/terraform.tfvars

10. Configure your Kubernetes client:

        $ gcloud container clusters get-credentials velor-$WORKSPACE --zone <region_zone_name> --project <project_name>
        # for example:
        $ gcloud container clusters get-credentials velor-$WORKSPACE --zone us-central1-a --project velor-fullnode

11. Check that your fullnode pods are now running (this may take a few minutes):

        $ kubectl get pods -n velor

12. Get your fullnode IP:

        $ kubectl get svc -o custom-columns=IP:status.loadBalancer.ingress -n velor

13. Check REST API, make sure the ledge version is increasing.

        $ curl http://<IP>

14. To verify the correctness of your FullNode, as outlined in the documentation (https://velor.dev/tutorials/run-a-fullnode/#verify-the-correctness-of-your-fullnode), you will need to set up a port-forwarding mechanism directly to the velor pod in one ssh terminal and test it in another ssh terminal

   * Set up the port-forwarding to the velor-fullnode pod.  Use `kubectl get pods -n velor` to get the name of the pod

         $ kubectl port-forward -n velor <pod-name> 9101:9101

   * Open a new ssh terminal.  Execute the following curl calls to verify the correctness

         $ curl -v http://0:9101/metrics 2> /dev/null | grep "velor_state_sync_version{type=\"synced\"}"

         $ curl -v http://0:9101/metrics 2> /dev/null | grep "velor_connections{direction=\"outbound\""

   * Exit port-forwarding when you are done by entering control-c in the terminal



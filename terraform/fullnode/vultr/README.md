Velor Fullnodes VULTR (https://www.vultr.com/) Deployment
==============================

This directory contains Terraform configs to deploy a public fullnode on VULTR.

These instructions assume that you have a functioning VULTR account. 
The default configuration will create a single node cluster with 4CPU/8GB and a automatically allocate and bind a persistant block storage (SSD) using VULTR-CSI (https://github.com/vultr/vultr-csi)


1. Install pre-requisites if needed:

   * Terraform 1.1.7: https://www.terraform.io/downloads.html
   * Docker: https://www.docker.com/products/docker-desktop
   * Kubernetes cli: https://kubernetes.io/docs/tasks/tools/
   
   Once you have a VULTR account, log into VULTR, go into ACCOUNT -> API and obtain your Personal Access Token.
   Configure the Access Control to whitelist the IP of the machine where you will run Terraform from.


2. Clone the velor-core repo and go to the terraform vultr folder.

         $ git clone https://github.com/velor-chain/velor-core.git

         $ cd velor-core/terraform/fullnode/vultr

3. Create a working directory for your configuration.  Copy the files you will change so it does not interfere with the cloned repo:

   * Choose a workspace name e.g. `devnet`. Note: this defines terraform workspace name, which in turn is used to form resource names.

         $ export WORKSPACE=devnet

   * Create a directory for the workspace

         $ mkdir -p ~/$WORKSPACE         

4. Change the cluster Name in `cluster.tf`

5. Configure cluster properties in `variables.tf`. 

    The most important variable is `api_key`, make sure you use the API key obtained in step 1. It will create a 1 machine with 4CPU/8GB in Frankfurt per default.

6. Apply the configuration with (it might take a while)
        
        $ terraform apply

7. Configure your Kubernetes client:

    Log in your VULTR account. Go to Products -> Kubernetes. Press  the 3 dots on the right side and choose "Manage".
    Press Download Configuration, it will download a YAML containing the access config to your cluster.

        $ export KUBECONFIG=~/vke...yaml

8. Check that your fullnode pods are now running (this may take a few minutes):

        $ kubectl get pods -n velor

9. Get your fullnode IP:

        $ kubectl get svc -o custom-columns=IP:status.loadBalancer.ingress -n velor

10. Check REST API, make sure the ledge version is increasing.

        $ curl http://<IP>

11. To verify the correctness of your FullNode, as outlined in the documentation (https://velor.dev/tutorials/run-a-fullnode/#verify-the-correctness-of-your-fullnode), you will need to set up a port-forwarding mechanism directly to the velor pod in one ssh terminal and test it in another ssh terminal

   * Set up the port-forwarding to the velor-fullnode pod.  Use `kubectl get pods -n velor` to get the name of the pod

         $ kubectl port-forward -n velor <pod-name> 9101:9101

   * Open a new ssh terminal.  Execute the following curl calls to verify the correctness

         $ curl -v http://0:9101/metrics 2> /dev/null | grep "velor_state_sync_version{type=\"synced\"}"

         $ curl -v http://0:9101/metrics 2> /dev/null | grep "velor_connections{direction=\"outbound\""

   * Exit port-forwarding when you are done by entering control-c in the terminal

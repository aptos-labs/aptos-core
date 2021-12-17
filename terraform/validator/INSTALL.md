Diem Single Validator Network Deployment
========================================

These instructions are for deploying a Diem blockchain network with one single validator node from scratch, this would be good for training new operators.

Deployment Package
------------------

The deployment package is in this directory. Clone the repo and checkout the branch corresponding to the version you want to deploy:

       $ git clone https://github.com/diem/diem
       $ cd diem/terraform/validator
       $ git checkout <version>

The release branch is usually named as `release-xx`, you can checkout to the latest one (e.g. `release-1.4`)

Once you have the correct validator directory version follow the instructions below.

Working with Diem Source Code
-----------------------------
If you haven't yet, it's highly recommended that you finish the "My first transaction" training: https://developers.diem.com/docs/my-first-transaction.

The training will guide you through the steps to setup environment and build Rust code, which we will use to build our operational tools.


Terraform with Vault
--------------------

1. Install pre-requisites if needed:

   * Terraform 1.0: https://www.terraform.io/downloads.html
   * Vault: https://www.vaultproject.io/downloads
   * Docker: https://www.docker.com/products/docker-desktop

2. Create a directory for your configuration:

   * Choose a workspace name e.g. testnet. Note: this defines terraform workspace name, which in turn is used to form resource names. If you are using a shared account please use a unique value here, e.g. `youralias-testnet`

         $ export WORKSPACE=testnet

   * Create a directory for the workspace

         $ mkdir -p ~/$WORKSPACE

3. Change into the appropriate directory for your cloud provider,
   make sure you setup credentials to access the cloud:
   * [aws][]: Amazon Web Services
   * [azure][]: Microsoft Azure
   * [gcp][]: Google Cloud Platform

         $ cd validator/<aws|azure|gcp>

4. Create a storage bucket for the Terraform state:
   * AWS: S3 bucket
   * GCP: Cloud Storage bucket
   * Azure: Blob storage container

5. Copy `backend.tfvars` to `~/$WORKSPACE/backend.tfvars` and edit to fill in your storage bucket details. For more detail on remote state see the Terraform documentation: https://www.terraform.io/docs/backends/index.html

       $ cp backend.tfvars ~/$WORKSPACE/backend.tfvars
       $ vi ~/$WORKSPACE/backend.tfvars

6. Initialise Terraform, providing your backend storage configuration:

       $ terraform init -backend-config ~/$WORKSPACE/backend.tfvars

7. Create a new Terraform workspace to isolate your environments:

   * Check the existing workspace, make sure the workspace name is not taken:

         $ terraform workspace list

   * Create the new workspace:

         $ terraform workspace new $WORKSPACE

8. Copy `terraform.tfvars` to `~/$WORKSPACE/validator.tfvars` and edit to set your validator name, SSH public key:

       $ cp terraform.tfvars ~/$WORKSPACE/validator.tfvars
       $ vi ~/$WORKSPACE/validator.tfvars


9. Apply the configuration, enabling the bastion host, which you will use later to access your vault. This will also create a `kubernetes.json` file with information about the Kubernetes cluster.

       $ terraform apply -var-file ~/$WORKSPACE/validator.tfvars -var bastion_enable=1

10. Configure your Kubernetes client:
    * aws:

          $ aws eks update-kubeconfig --name diem-$WORKSPACE

    * azure:

          $ az aks get-credentials --resource-group diem-$WORKSPACE --name diem-$WORKSPACE

    * gcp:

          $ gcloud container clusters get-credentials diem-$WORKSPACE --zone us-central1-a --project diem

11. Find the private IP of one of the `vault` instances, and the public IP of the `bastion` instance.
    * aws:

          $ terraform state show 'aws_instance.bastion[0]' | grep public_dns
          $ aws ec2 describe-instances --filters "Name=tag:Name,Values=diem-$WORKSPACE/vault" --query "Reservations[*].Instances[*].PrivateDnsName" --output=text

    * gcp:

          $ terraform state show 'google_compute_instance.bastion[0]' | grep nat_ip
          $ gcloud --project <gcp project> compute instances list

    * azure:

          $ terraform state show 'azurerm_linux_virtual_machine.bastion[0]' | grep public_ip
          $ az vmss nic list-vm-nics -g diem-$WORKSPACE --vmss-name diem-$WORKSPACE-vault --instance-id 1  | grep privateIp

        * alternatively, cut-n-past this:

```sh
                public_ip=$(terraform state show 'azurerm_linux_virtual_machine.bastion[0]' | egrep 'public_ip_address\W' | awk -F'"' '{print $2}')
                echo public_ip=$public_ip
                private_ip=$(az vmss nic list -g diem-$WORKSPACE --vmss-name diem-$WORKSPACE-vault --query '[].ipConfigurations[].privateIpAddress' --output tsv)
                echo private_ip=$private_ip
```

12. SSH to the vault host via the bastion host, setting up a tunnel to connect to the Vault API.

* **Keep this running in another window.**
* `~/.ssh/ec2_rsa` is the private key you created in step 8.
* For Azure replace "ec2-user" with "az-user"
* for GCP replace with the username you configured in `ssh_keys`.

        $ ssh -o ProxyCommand="ssh -i ~/.ssh/ec2_rsa ec2-user@<bastion public_dns> proxy %h %p" -L 8200:localhost:8200 -i ~/.ssh/ec2_rsa ec2-user@<vault private_dns>


13. Initialise Vault **from the vault host**.

        [ec2-user@vault ~]$ VAULT_CACERT=/etc/vault/vault.ca vault operator init

14. Switch back to your local host, save the Initial Root Token in a file, you'll need it to manage Vault:

        $ vi ~/$WORKSPACE/vault.token

15. Change to the [vault-init][] directory:

        $ cd ../vault-init

16. Copy `backend.tf.example` to `backend.tf` and edit to uncomment the appropriate backend for your cloud provider.

        $ cp backend.tf.example backend.tf
        $ vi backend.tf

17. Copy `backend.tfvars` to `~/$WORKSPACE/backend-vault.tfvars` and edit to fill in your storage bucket details.

        $ cp backend.tfvars ~/$WORKSPACE/backend-vault.tfvars
        $ vi ~/$WORKSPACE/backend-vault.tfvars

18. Initialise Terraform, providing your backend storage configuration:

        $ terraform init -backend-config ~/$WORKSPACE/backend-vault.tfvars

19. Specify your Vault server information (replace "aws" with "gcp" or "azure" as necessary):

        $ export VAULT_ADDR=https://localhost:8200
        $ export VAULT_CACERT=$PWD/../<aws|azure|gcp>/$WORKSPACE-vault.ca
        $ export VAULT_TOKEN=<vault root token>

     Check your `VAULT_CACERT` env (`echo $VAULT_CERT`), make sure it's the full path to the vault CA certificate file.
     `<vault root token>` is the token you generated in step 13.

20. Rename `diem-root.tf.example` to `diem-root.tf`, this will add `diem-root` key and `treasury_compliance` key in your vault for this test.

        $ cp diem-root.tf.example diem-root.tf

21. Apply the configuration, providing your Kubernetes cluster information:

        $ terraform apply -var-file ../<aws|azure|gcp>/$WORKSPACE-kubernetes.json

22. Verify you have access to the vault:

        $ vault kv list transit/keys

    You should be able to see a list of keys, e.g. `diem__root`, `diem__operator`

23. Change into your configuration directory:

        $ cd ~/$WORKSPACE

24. Set the operator name:

        $ export OPERATOR=<your_name>

25. Collect the endpoints of the `-validator-lb` and `-fullnode-lb` services:

    * If using the included DNS setup:

          $ cd ~/diem/terraform/validator/<aws|azure|gcp>
          $ export VALIDATOR_ADDRESS="$(terraform output validator_endpoint)"
          $ export FULLNODE_ADDRESS="$(terraform output fullnode_endpoint)"
          $ cd -

    * Otherwise on AWS:

          $ export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-diem-validator-validator-lb -o go-template='/dns4/{{(index .status.loadBalancer.ingress 0).hostname}}/tcp/{{(index .spec.ports 0).port}}')"
          $ export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-diem-validator-fullnode-lb -o go-template='/dns4/{{(index .status.loadBalancer.ingress 0).hostname}}/tcp/{{(index .spec.ports 0).port}}')"

    * Otherwise on GCP and Azure:

          $ export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-diem-validator-validator-lb -o go-template='/ip4/{{(index .status.loadBalancer.ingress 0).ip}}/tcp/{{(index .spec.ports 0).port}}')"
          $ export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-diem-validator-fullnode-lb -o go-template='/ip4/{{(index .status.loadBalancer.ingress 0).ip}}/tcp/{{(index .spec.ports 0).port}}')"

26. Build the Diem genesis tool binary:

        $ git clone https://github.com/diem/diem
        $ cd diem
        $ git checkout <version>
        $ cargo build -p diem-genesis-tool
        $ cargo run -p diem-genesis-tool -- help

    Follow [My first transaction][] to setup the environment for building Rust code.

27. Use the genesis script to generate keys, create waypoints, and compile the genesis blob for your blockchain:

        $ cd ~/$WORKSPACE
        $ export GENESIS=~/diem/target/debug/diem-genesis-tool
        $ export BACKEND="backend=vault;server=https://localhost:8200;ca_certificate=<full path to $WORKSPACE-vault.ca>;token=vault.token;namespace=diem"
        $ ~/diem/scripts/genesis-single.sh $GENESIS $BACKEND $VALIDATOR_ADDRESS $FULLNODE_ADDRESS

28. Insert the genesis blob into the Kubernetes configmap for the chain era (defined in [helm/values.yaml][]):

        $ kubectl create configmap $WORKSPACE-diem-validator-genesis-e<chain_era_number> --from-file=genesis.blob=genesis.blob

    change `<chain_era_number>` with current chain era number defined in [helm/values.yaml][].

29. Check that your pods are now running (this may take a few minutes):

        $ kubectl get pods

30. To access the monitoring dashboard run the following:

        $ kubectl port-forward $WORKSPACE-diem-validator-monitoring-0 3000

    And then load http://localhost:3000/d/validator


Destroy existing deployment
--------------------

1. Make sure you select the terraform workspace you want to destroy:

         $ cd ~/diem/terraform/validator/<cloud>
         $ export WORKSPACE=testnet
         $ terraform workspace select $WORKSPACE

2. Enable destroying of protected resources in this deployment.

        $ sed -i 's/prevent_destroy = true/prevent_destroy = false/' cluster.tf vault.tf

3. Destroy the deployment

         $ terraform destroy -var-file ~/$WORKSPACE/validator.tfvars

4. Restore the code protecting resources.

        $ sed -i 's/prevent_destroy = false/prevent_destroy = true/' cluster.tf vault.tf

[aws]: aws/
[gcp]: gcp/
[azure]: azure/
[vault-init]: vault-init/
[helm/values.yaml]: helm/values.yaml
[My first transaction]: https://developers.diem.com/docs/core/my-first-transaction

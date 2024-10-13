import yaml
from kubernetes import client, config
from kubernetes.client.rest import ApiException
import copy

NUM_OF_WORKERS = 2

# Load Kubernetes configuration
config.load_kube_config()  # If you're using a kubeconfig file for authentication (e.g., on your local machine)

# If running inside a Kubernetes cluster, use this instead:
# config.load_incluster_config()

# Load the worker YAML from the file
with open("replay-verify-worker-template.yaml", "r") as f:
    pod_manifest = yaml.safe_load(f)

# Create the Kubernetes API client
api_instance = client.CoreV1Api()

try:
    # Create a PVC in the default namespace
    
    # Create Pods in the default namespace
    for i in range(NUM_OF_WORKERS):
        pod_copy = copy.deepcopy(pod_manifest)  # Create a deep copy for each pod
        pod_copy["metadata"]["name"] = f"replay-verify-worker-{i}"  # Unique name for each pod
        pod_copy["spec"]["containers"][0]["name"] = f"replay-verify-worker-{i}"
        
        response = api_instance.create_namespaced_pod(
            namespace="default", body=pod_copy
        )
        print(f"Pod {i} created. Status: {response.metadata.name}")
except ApiException as e:
    print(f"Error creating pod: {e}")

from kubernetes import client, config

def init():
    print("Initializing the cluster ...")
    config.load_kube_config()

def list_nodes():
    v1 = client.CoreV1Api()
    print("Listing nodes:")
    ret = v1.list_node(watch=False)
    for node in ret.items:
        print(f"Node Name: {node.metadata.name}")

def create_pod(pod_name, image_name, namespace="default"):
    v1 = client.CoreV1Api()
    
    # Define the pod spec
    pod = client.V1Pod(
        metadata=client.V1ObjectMeta(name=pod_name),
        spec=client.V1PodSpec(
            containers=[client.V1Container(
                name=pod_name,
                image=image_name,
                )],
            restart_policy="Never"
        )
    )
    
    try:
        # Create the pod
        response = v1.create_namespaced_pod(namespace=namespace, body=pod)
        print(f"Pod {pod_name} created. Status: {response.status.phase}")
    except client.exceptions.ApiException as e:
        print(f"Exception when creating pod {pod_name}: {e}")

def create_pods(image_name, count=2, namespace="default"):
    for i in range(count):
        pod_name = f"pod-{i+1}"
        create_pod(pod_name, image_name, namespace)

if __name__ == '__main__':
    print("Starting the script...")
    init()
    list_nodes()
    
    # Create 10 pods using the aptoslabs/tools:nightly image
    create_pods("aptoslabs/tools:nightly", count=2, namespace="default")

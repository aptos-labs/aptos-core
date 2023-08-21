from typing import Union, List
from kubernetes import client, config
from kubernetes.client import ApiException  # type: ignore
from kubernetes.stream import stream
from abc import ABC, abstractmethod
from test_framework.logging import log
import time
from typing import Any

KubernetesResource = Union[
    client.V1Namespace,
    client.V1Service,
    client.V1StatefulSet,
    client.V1ConfigMap,
    client.V1Secret,
    client.V1Pod,
    client.V1PersistentVolumeClaim,
]


class Kubernetes(ABC):
    @abstractmethod
    def create_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> KubernetesResource:
        pass

    @abstractmethod
    def delete_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> bool:
        pass

    @abstractmethod
    def get_resources(
        self, type: type, namespace: str = "default"
    ) -> List[KubernetesResource]:
        pass

    @abstractmethod
    def get_pod_list(self, namespace: str) -> client.V1PodList:
        pass

    @abstractmethod
    def delete_namespace(self, namespace: str, wait_deletion: bool) -> bool:
        pass

    @abstractmethod
    def scale_stateful_set(
        self, namespace: str, statefulset_name: str, replicas: int
    ) -> client.V1StatefulSet:
        pass

    @abstractmethod
    def patch_resource(
        self,
        type: type,
        name: str,
        patch_data: Any,
        namespace: str = "default",
    ) -> KubernetesResource:
        pass

    @abstractmethod
    def exec_command(
        self,
        namespace: str,
        pod_name: str,
        command: List[str],
    ) -> str:
        pass


class LiveKubernetes(Kubernetes):
    def __init__(self):
        self.TIMEOUT_LIMIT = 3600
        try:
            current_context: str = config.list_kube_config_contexts()[1]["context"]["cluster"]  # type: ignore
            cluster_name: str = current_context.split("/")[-1]  # type: ignore
            log.info(f': Operating in the cluster named "{cluster_name}"')  # type: ignore
        except:
            raise ApiException(status=400, reason="NO KUBERNETES CLUSTER FOUND.")

    def create_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> KubernetesResource:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        apps_v1_api = client.AppsV1Api()
        if not kubernetes_object.metadata or not kubernetes_object.metadata.name:
            raise ApiException(
                status=400,
                reason="Cannot create a k8s resource without metadata or name!",
            )
        self._verify_k8s_obj_name(namespace)
        resource_name: str = kubernetes_object.metadata.name
        self._verify_k8s_obj_name(resource_name)
        if isinstance(kubernetes_object, client.V1Namespace):
            return core_v1_api.create_namespace(kubernetes_object)
        elif isinstance(kubernetes_object, client.V1Service):
            return core_v1_api.create_namespaced_service(
                namespace=namespace, body=kubernetes_object
            )
        elif isinstance(kubernetes_object, client.V1StatefulSet):
            return apps_v1_api.create_namespaced_stateful_set(
                namespace=namespace, body=kubernetes_object
            )
        elif isinstance(kubernetes_object, client.V1ConfigMap):
            return core_v1_api.create_namespaced_config_map(
                namespace=namespace, body=kubernetes_object
            )
        elif isinstance(kubernetes_object, client.V1Secret):  # type: ignore
            return core_v1_api.create_namespaced_secret(
                namespace=namespace, body=kubernetes_object
            )
        elif isinstance(kubernetes_object, client.V1PersistentVolumeClaim):  # type: ignore
            return core_v1_api.create_namespaced_persistent_volume_claim(
                namespace=namespace, body=kubernetes_object
            )
        elif isinstance(kubernetes_object, client.V1Pod):  # type: ignore
            return core_v1_api.create_namespaced_pod(
                namespace=namespace, body=kubernetes_object
            )
        else:
            raise NotImplemented("This resource type is not implemented!")

    def delete_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> bool:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        apps_v1_api = client.AppsV1Api()
        if not kubernetes_object.metadata or not kubernetes_object.metadata.name:
            raise ApiException(
                status=400,
                reason="Cannot delete a k8s resource without metadata or name!",
            )
        self._verify_k8s_obj_name(namespace)
        resource_name: str = kubernetes_object.metadata.name
        self._verify_k8s_obj_name(resource_name)
        if isinstance(kubernetes_object, client.V1Namespace):
            try:
                core_v1_api.delete_namespace(
                    name=namespace, body=client.V1DeleteOptions()
                )
            except Exception as exception:
                log.error(f'Failed deleting the namespace "{namespace}""!')
                return False
        elif isinstance(kubernetes_object, client.V1Service):
            try:
                core_v1_api.delete_namespaced_service(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(f'Failed deleting the service "{resource_name}""!')
                return False
        elif isinstance(kubernetes_object, client.V1StatefulSet):
            try:
                apps_v1_api.delete_namespaced_stateful_set(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(f'Failed deleting the statefulset "{resource_name}""!')
                return False
        elif isinstance(kubernetes_object, client.V1ConfigMap):
            try:
                core_v1_api.delete_namespaced_config_map(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(f'Failed deleting the configmap "{resource_name}""!')
                return False
        elif isinstance(kubernetes_object, client.V1Secret):  # type: ignore
            try:
                core_v1_api.delete_namespaced_secret(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(f'Failed deleting the secret "{resource_name}""!')
                return False
        elif isinstance(kubernetes_object, client.V1PersistentVolumeClaim):  # type: ignore
            try:
                core_v1_api.delete_namespaced_persistent_volume_claim(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(
                    f'Failed deleting the persistent volume claim "{resource_name}""!'
                )
                return False
        elif isinstance(kubernetes_object, client.V1Pod):  # type: ignore
            try:
                core_v1_api.delete_namespaced_pod(
                    name=resource_name,
                    namespace=namespace,
                    body=client.V1DeleteOptions(),
                )
            except Exception as exception:
                log.error(f'Failed deleting the pod "{resource_name}""!')
                return False
        else:
            raise NotImplemented(
                "Delete operation on this resource type is not implemented!"
            )

        return True

    def get_resources(
        self, type: type, namespace: str = "default"
    ) -> List[KubernetesResource]:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        apps_v1_api = client.AppsV1Api()
        self._verify_k8s_obj_name(namespace)
        if type == client.V1Namespace:
            return list(core_v1_api.list_namespace().items)
        if type == client.V1Service:
            return list(core_v1_api.list_namespaced_service(namespace=namespace).items)
        if type == client.V1StatefulSet:
            return list(
                apps_v1_api.list_namespaced_stateful_set(namespace=namespace).items
            )
        if type == client.V1ConfigMap:
            return list(
                core_v1_api.list_namespaced_config_map(namespace=namespace).items
            )
        if type == client.V1Secret:
            return list(core_v1_api.list_namespaced_secret(namespace=namespace).items)
        if type == client.V1Pod:
            return list(core_v1_api.list_namespaced_pod(namespace=namespace).items)
        else:
            raise NotImplementedError("This resource type is not implemented!")

    def get_pod_list(self, namespace: str) -> client.V1PodList:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        pods = core_v1_api.list_namespaced_pod(namespace=namespace)
        return pods

    def scale_stateful_set(
        self,
        namespace: str,
        statefulset_name: str,
        replicas: int,
        wait_scaling: bool = True,
    ) -> client.V1StatefulSet:
        config.load_kube_config()  # type:ignore
        apps_v1_api = client.AppsV1Api()
        statefulset = apps_v1_api.read_namespaced_stateful_set(
            name=statefulset_name, namespace=namespace
        )
        if statefulset.spec:
            statefulset.spec.replicas = replicas
            apps_v1_api.patch_namespaced_stateful_set(
                name=statefulset_name, namespace=namespace, body=statefulset
            )
            if wait_scaling:
                return self._wait_for_scaling(namespace, statefulset_name, replicas)
            else:
                return statefulset
        else:
            raise ApiException(status=400, reason="NO STATEFULSET SPEC FOUND.")

    def delete_namespace(self, namespace: str, wait_deletion: bool) -> bool:
        # TODO Deprecate this method, merge with delete_resource
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        try:
            core_v1_api.delete_namespace(name=namespace, body=client.V1DeleteOptions())
            if wait_deletion:
                self._wait_namespace_deletion(namespace)
        except Exception as exception:
            log.error(f'Failed deleting the namespace "{namespace}""!')
            log.error(f"Error status: {exception.status}")  # type: ignore
            log.error(f"Error reason: {exception.reason}")  # type: ignore
            return False
        return True

    def patch_resource(
        self,
        type: type,
        name: str,
        patch_data: Any,
        namespace: str = "default",
    ) -> KubernetesResource:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        apps_v1_api = client.AppsV1Api()
        if type == client.V1ConfigMap:
            return core_v1_api.patch_namespaced_config_map(
                name=name, namespace=namespace, body=patch_data
            )
        elif type == client.V1StatefulSet:
            return apps_v1_api.patch_namespaced_stateful_set(
                name=name, namespace=namespace, body=patch_data
            )
        else:
            raise NotImplementedError("This resource type is not implemented!")

    def exec_command(
        self,
        namespace: str,
        pod_name: str,
        command: List[str],
    ) -> str:
        config.load_kube_config()  # type:ignore
        core_v1_api = client.CoreV1Api()
        resp = core_v1_api.read_namespaced_pod(name=pod_name, namespace=namespace)
        if resp.status.phase == "Running":  # type:ignore
            exec_command = [
                "/bin/sh",
                "-c",
                " ".join(command),
            ]
            try:
                resp = stream(
                    core_v1_api.connect_get_namespaced_pod_exec,
                    pod_name,
                    namespace,
                    command=exec_command,
                    stderr=True,
                    stdin=False,
                    stdout=True,
                    tty=False,
                )
                return resp
            except Exception as e:
                raise ApiException(
                    status=400, reason="Error executing command: " + str(e)
                )
        else:
            raise ApiException(status=400, reason="POD NOT RUNNING.")

    def _wait_for_scaling(
        self, namespace: str, statefulset_name: str, target_replicas: int
    ) -> client.V1StatefulSet:
        config.load_kube_config()  # type:ignore
        apps_v1_api = client.AppsV1Api()
        timeout_counter = 0
        while True:
            if timeout_counter > self.TIMEOUT_LIMIT:
                raise TimeoutError(
                    f"Timeout while scaling the statefulset {statefulset_name} in the namespace {namespace}!"
                )
            statefulset = apps_v1_api.read_namespaced_stateful_set(
                name=statefulset_name, namespace=namespace
            )
            if statefulset.status and statefulset.status.replicas == target_replicas:
                return statefulset
            timeout_counter += 3
            time.sleep(3)

    def _wait_namespace_deletion(self, namespace: str) -> None:
        core_v1_api = client.CoreV1Api()
        timeout_counter = 0
        while True:
            if timeout_counter > self.TIMEOUT_LIMIT:
                raise TimeoutError(f"Timeout while deleting the namespace {namespace}!")
            try:
                core_v1_api.read_namespace(name=namespace)
                timeout_counter += 5
                time.sleep(5)
            except ApiException as e:  # type: ignore
                if e.status == 404:  # type: ignore
                    break
                else:
                    raise e

    def _verify_k8s_obj_name(self, name: str):
        if len(name) > 63:
            raise ApiException(
                status=400,
                reason="K8s object length exceeds the maximum allowed limit of 63 characters.",
            )


class SpyKubernetes(Kubernetes):
    def __init__(self):
        # self.namespaces: set[str] = set()
        self.namespaces: dict[str, client.V1Namespace] = dict()
        self.namespaced_resource_dictionary: dict[
            str, dict[int, dict[str, KubernetesResource]]
        ] = {}  # Namespace -> Resource Type Hash -> Resource Name -> Resource Object
        default_namespace = client.V1Namespace()
        default_namespace.metadata = client.V1ObjectMeta(name="default")
        self.namespaces["default"] = default_namespace
        self.namespaced_resource_dictionary["default"] = dict()
        self.patches: List[Any] = []
        self.exec_commands: dict[str, dict[str, List[str]]] = dict()
        self.exec_commands["default"] = dict()

    def create_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> KubernetesResource:
        if not kubernetes_object.metadata or not kubernetes_object.metadata.name:
            raise ApiException(
                status=400,
                reason="Cannot create a k8s resource without metadata or name!",
            )
        self._verify_k8s_obj_name(namespace)
        resource_name: str = kubernetes_object.metadata.name
        self._verify_k8s_obj_name(resource_name)
        if isinstance(kubernetes_object, client.V1Namespace):
            if kubernetes_object.metadata.name in self.namespaces:
                raise ApiException(
                    status=409,
                    reason=f'The namespace with the name "{kubernetes_object.metadata.name}" already exists!',
                )
            self.namespaces[resource_name] = kubernetes_object
            self.namespaced_resource_dictionary[resource_name] = dict()
            return kubernetes_object
        else:
            return self._create_resource_helper(
                namespace, resource_name, kubernetes_object
            )

    def delete_resource(
        self, kubernetes_object: KubernetesResource, namespace: str = "default"
    ) -> bool:
        if not kubernetes_object.metadata or not kubernetes_object.metadata.name:
            raise ApiException(
                status=400,
                reason="Cannot delete a k8s resource without metadata or name!",
            )
        self._verify_k8s_obj_name(namespace)
        resource_name: str = kubernetes_object.metadata.name
        self._verify_k8s_obj_name(resource_name)
        if isinstance(kubernetes_object, client.V1Namespace):
            if not resource_name in self.namespaces:
                raise ApiException(
                    status=400,
                    reason=f'The namespace with the name "{resource_name}" does not exist!',
                )
            self.namespaces.pop(resource_name)
            self.namespaced_resource_dictionary.pop(resource_name)
            return True
        else:
            return self._delete_resource_helper(
                namespace, resource_name, kubernetes_object
            )

    def get_resources(
        self, type: type, namespace: str = "default"
    ) -> List[KubernetesResource]:
        self._check_namespace_exists(namespace)
        if type == client.V1Namespace:
            return list(self.namespaces.values())
        resource_type: int = hash(type)
        if not resource_type in self.namespaced_resource_dictionary[namespace]:
            return []
        resources: dict[str, KubernetesResource] = self.namespaced_resource_dictionary[
            namespace
        ][resource_type]
        return list(resources.values())

    def get_pod_list(self, namespace: str) -> client.V1PodList:
        self._check_namespace_exists(namespace)
        pod_map = self.namespaced_resource_dictionary[namespace][hash(client.V1Pod)]
        namespaced_pods: List[client.V1Pod] = []
        for pod_name in pod_map:
            namespaced_pods.append(pod_map[pod_name])  # type: ignore
        return client.V1PodList(items=namespaced_pods)

    def delete_namespace(self, namespace: str, wait_deletion: bool) -> bool:
        self._check_namespace_exists(namespace)
        self.namespaced_resource_dictionary.pop(namespace)
        self.namespaces.pop(namespace)
        return True

    def scale_stateful_set(
        self, namespace: str, statefulset_name: str, replicas: int
    ) -> client.V1StatefulSet:
        self._check_namespace_exists(namespace)
        if (
            not hash(client.V1StatefulSet)
            in self.namespaced_resource_dictionary[namespace]
        ):
            raise ApiException(
                status=400,
                reason="This namespace does not have any statefulsets!",
            )
        statefulset_map = self.namespaced_resource_dictionary[namespace][
            hash(client.V1StatefulSet)
        ]

        if not statefulset_name in statefulset_map:
            raise ApiException(
                status=400,
                reason="This statefulset does not exit!",
            )
        statefulset = statefulset_map[statefulset_name]
        statefulset.status.replicas = replicas  # type: ignore
        return statefulset  # type: ignore

    def patch_resource(
        self,
        type: type,
        name: str,
        patch_data: Any,
        namespace: str = "default",
    ) -> KubernetesResource:
        self._check_namespace_exists(namespace)
        if not hash(type) in self.namespaced_resource_dictionary[namespace]:
            raise ApiException(
                status=400,
                reason="This namespace does not have any resources of this type!",
            )
        resource_map = self.namespaced_resource_dictionary[namespace][hash(type)]
        if not name in resource_map:
            raise ApiException(
                status=400,
                reason="This resource does not exit!",
            )
        resource = resource_map[name]
        self.patches.append(patch_data)
        return resource

    def exec_command(
        self,
        namespace: str,
        pod_name: str,
        command: List[str],
    ) -> str:
        self._check_namespace_exists(namespace)
        if not hash(client.V1Pod) in self.namespaced_resource_dictionary[namespace]:
            raise ApiException(
                status=400,
                reason="This namespace does not have any pods!",
            )
        pod_map = self.namespaced_resource_dictionary[namespace][hash(client.V1Pod)]
        if not pod_name in pod_map:
            raise ApiException(
                status=400,
                reason="This pod does not exit!",
            )

        #
        # Add the command to the pods command history.
        command_string = " ".join(command)
        if not pod_name in self.exec_commands[namespace]:
            self.exec_commands[namespace][pod_name] = []
        self.exec_commands[namespace][pod_name].append(command_string)
        return " ".join(command)

    def _check_namespace_exists(self, namespace: str):
        if not namespace in self.namespaces:
            raise ApiException(
                status=400,
                reason="This namespace does not exit!",
            )

    def _verify_k8s_obj_name(self, name: str):
        if len(name) > 63:
            raise ApiException(
                status=400,
                reason="K8s object length exceeds the maximum allowed limit of 63 characters.",
            )

    def _create_resource_helper(
        self,
        namespace: str,
        resource_name: str,
        resource: KubernetesResource,
    ) -> KubernetesResource:
        resource_type: int = hash(type(resource))
        self._check_namespace_exists(namespace)
        resource_types: dict[
            int, dict[str, KubernetesResource]
        ] = self.namespaced_resource_dictionary[namespace]
        if not resource_type in resource_types:
            resource_types[resource_type] = dict()
        resources: dict[str, KubernetesResource] = resource_types[resource_type]
        if resource_name in resources:
            log.error(
                f'This {resource_type} named "{resource_name}" already exists in this namespace "{namespace}"!'
            )
            raise ApiException(
                status=409,
                reason=f'The namespace with the name "{resource_name}" already exists!',
            )
        resources[resource_name] = resource
        return resource

    def _delete_resource_helper(
        self,
        namespace: str,
        resource_name: str,
        resource: KubernetesResource,
    ) -> bool:
        resource_type: int = hash(type(resource))
        self._check_namespace_exists(namespace)
        resource_types: dict[
            int, dict[str, KubernetesResource]
        ] = self.namespaced_resource_dictionary[namespace]
        if not resource_type in resource_types:
            resource_types[resource_type] = dict()
        resources: dict[str, KubernetesResource] = resource_types[resource_type]
        if not resource_name in resources:
            log.error(
                f'This {resource_type} named "{resource_name}" does not exist in this namespace "{namespace}"!'
            )
            raise ApiException(
                status=409,
                reason=f'The namespace with the name "{resource_name}" does not exist!',
            )
        resources.pop(resource_name)
        return True

import unittest
from .kubernetes import SpyKubernetes, KubernetesResource
from kubernetes import client
from kubernetes.client import ApiException  # type: ignore


class SpyTests(unittest.TestCase):
    def setUp(self):
        self.kubernetes: SpyKubernetes = SpyKubernetes()
        self.default_namespace_name: str = "default"

    def test_no_metadata(self) -> None:
        service = client.V1Service()
        with self.assertRaises(ApiException):  # type: ignore
            self.kubernetes.create_resource(service)

    def test_no_valid_namespace(self) -> None:
        service: KubernetesResource = client.V1Service(
            api_version="v1",
            kind="Service",
            metadata=client.V1ObjectMeta(name=f"test"),
            spec=client.V1ServiceSpec(),
        )

        self.assertEqual(service, self.kubernetes.create_resource(service))

        with self.assertRaises(ApiException):  # type: ignore
            self.kubernetes.create_resource(service, "namespace")

    def test_creating_resources(self):
        namespace: KubernetesResource = client.V1Namespace(
            metadata=client.V1ObjectMeta(name=f"test_space")
        )
        try:
            self.kubernetes.create_resource(namespace)
        except ApiException as exc:  # type: ignore
            self.fail("Creating a valid namespace failed!")

        service: KubernetesResource = client.V1Service(
            metadata=client.V1ObjectMeta(name=f"test_service")
        )
        try:
            self.kubernetes.create_resource(service, self.default_namespace_name)
        except ApiException as exc:  # type: ignore
            self.fail("Creating a valid service failed!")
        with self.assertRaises(ApiException):  # type: ignore
            self.kubernetes.create_resource(service, self.default_namespace_name)
        service_2: KubernetesResource = client.V1Service(
            metadata=client.V1ObjectMeta(name=f"test_service_2")
        )
        try:
            self.kubernetes.create_resource(service_2, "test_space")
        except ApiException as exc:  # type: ignore
            self.fail("Creating a valid service failed!")
        self.assertEqual(len(self.kubernetes.namespaces), 2)
        self.assertEqual(
            service,
            self.kubernetes.namespaced_resource_dictionary["default"][
                hash(client.V1Service)
            ]["test_service"],
        )
        self.assertEqual(
            service_2,
            self.kubernetes.namespaced_resource_dictionary["test_space"][
                hash(client.V1Service)
            ]["test_service_2"],
        )

    def test_creating_long_names(self):
        long_name: str = "x" * 64
        namespace: KubernetesResource = client.V1Namespace(
            metadata=client.V1ObjectMeta(name=long_name)
        )
        try:
            self.kubernetes.create_resource(namespace)
        except ApiException as exc:  # type: ignore
            self.assertEqual("K8s object length exceeds the maximum allowed limit of 63 characters.", exc.reason)  # type: ignore
        service: KubernetesResource = client.V1Namespace(
            metadata=client.V1ObjectMeta(name=long_name)
        )
        try:
            self.kubernetes.create_resource(service, self.default_namespace_name)
        except ApiException as exc:  # type: ignore
            self.assertEqual("K8s object length exceeds the maximum allowed limit of 63 characters.", exc.reason)  # type: ignore

    def test_get_pod_list(self):
        self.kubernetes.namespaced_resource_dictionary["default"][
            hash(client.V1Pod)
        ] = dict()
        with self.assertRaises(ApiException):  # type: ignore
            self.kubernetes.get_pod_list("does_not_exist")
        self.assertEqual(0, len(self.kubernetes.get_pod_list("default").items))
        pod1: KubernetesResource = client.V1Pod(
            metadata=client.V1ObjectMeta(name="pod1"),
        )
        pod2: KubernetesResource = client.V1Pod(
            metadata=client.V1ObjectMeta(name="pod2"),
        )
        self.kubernetes.namespaced_resource_dictionary["default"][hash(client.V1Pod)][
            "pod1"
        ] = pod1
        self.kubernetes.namespaced_resource_dictionary["default"][hash(client.V1Pod)][
            "pod2"
        ] = pod2
        pod_list = client.V1PodList(items=[pod1, pod2])
        self.assertEqual(pod_list, self.kubernetes.get_pod_list("default"))

    def test_delete_namespace(self):
        namespace: KubernetesResource = client.V1Namespace(
            metadata=client.V1ObjectMeta(name="pangu-name")
        )
        self.kubernetes.create_resource(namespace)
        with self.assertRaises(ApiException):  # type: ignore
            self.kubernetes.delete_namespace("doesnt-exist", False)
        try:
            self.kubernetes.delete_namespace("pangu-name", False)
        except ApiException as exc:  # type: ignore
            self.fail("Valid namespace deletion failed!")

    def test_get_resources(self):
        namespace: KubernetesResource = client.V1Namespace(
            metadata=client.V1ObjectMeta(name="pangu-name")
        )
        self.kubernetes.create_resource(namespace)
        with self.assertRaises(ApiException):  # type:ignore
            self.kubernetes.get_resources(client.V1Pod, "doesnt-exist")
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1Pod, "pangu-name"))
        )
        pod1: KubernetesResource = client.V1Pod(
            metadata=client.V1ObjectMeta(name="pod1"),
        )
        pod2: KubernetesResource = client.V1Pod(
            metadata=client.V1ObjectMeta(name="pod2"),
        )
        self.kubernetes.create_resource(pod1, "pangu-name")
        self.kubernetes.create_resource(pod2, "pangu-name")

        self.assertEqual(
            2, len(self.kubernetes.get_resources(client.V1Pod, "pangu-name"))
        )
        self.assertEqual(0, len(self.kubernetes.get_resources(client.V1Pod, "default")))
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1Service, "pangu-name"))
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1Service, "default"))
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1StatefulSet, "pangu-name"))
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1StatefulSet, "default"))
        )
        self.assertEqual(
            0,
            len(
                self.kubernetes.get_resources(
                    client.V1PersistentVolumeClaim, "pangu-name"
                )
            ),
        )
        self.assertEqual(
            0,
            len(
                self.kubernetes.get_resources(client.V1PersistentVolumeClaim, "default")
            ),
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1ConfigMap, "pangu-name"))
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1ConfigMap, "default"))
        )
        self.assertEqual(
            0, len(self.kubernetes.get_resources(client.V1Secret, "pangu-name"))
        )

    def test_scale_stateful_set(self):
        """tests scale_stateful_set"""
        stateful_set: KubernetesResource = client.V1StatefulSet(
            metadata=client.V1ObjectMeta(name="stateful-set"),
            status=client.V1StatefulSetStatus(replicas=0),
        )
        if stateful_set.status is None:
            self.fail("status is None")
        self.kubernetes.create_resource(stateful_set, "default")
        self.assertEqual(0, stateful_set.status.replicas)
        self.kubernetes.scale_stateful_set("default", "stateful-set", 1)
        self.assertEqual(1, stateful_set.status.replicas)
        self.kubernetes.scale_stateful_set("default", "stateful-set", 0)
        self.assertEqual(0, stateful_set.status.replicas)
        self.kubernetes.scale_stateful_set("default", "stateful-set", 3)
        self.assertEqual(3, stateful_set.status.replicas)

    def test_patch_configmap(self):
        """tests patch_configmap"""
        config_map: KubernetesResource = client.V1ConfigMap(
            metadata=client.V1ObjectMeta(name="config-map"),
            data={"key": "value"},
        )
        self.kubernetes.create_resource(config_map, "default")
        patch_data = {"data": {"key": "new_value"}}
        self.assertEqual("value", config_map.data["key"])  # type: ignore
        self.kubernetes.patch_resource(
            client.V1ConfigMap, "config-map", patch_data, "default"
        )
        self.assertEqual(self.kubernetes.patches[0], patch_data)

    def test_exec_command(self):
        """tests exec_command"""
        pod: KubernetesResource = client.V1Pod(
            metadata=client.V1ObjectMeta(name="pod"),
        )
        self.kubernetes.create_resource(pod, "default")
        self.kubernetes.exec_command("default", "pod", ["command"])
        self.assertEqual("command", self.kubernetes.exec_commands["default"]["pod"][0])

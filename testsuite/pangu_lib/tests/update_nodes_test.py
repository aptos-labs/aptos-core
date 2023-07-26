from test_framework.kubernetes import SpyKubernetes
from test_framework.filesystem import SpyFilesystem, LocalFilesystem
from test_framework.shell import SpyShell
from pangu_lib.util import SystemContext
import unittest
from kubernetes.client import ApiException  # type: ignore
from pangu_lib.node_commands.restart_node import *
from pangu_lib.node_commands.start_node import *
from pangu_lib.node_commands.stop_node import *
from pangu_lib.testnet_commands.update_nodes import *


class testnet_update_nodes_test(unittest.TestCase):
    def setUp(self) -> None:
        self.maxDiff = None
        self.pangu_config_no_vfn = b"blueprints:\n \n nodebp:\n    nodes_persistent_volume_claim_size: size\n    validator_storage_class_name: standard\n    vfn_storage_class_name: standard\n    validator_config_path: /path/to/config.yaml\n    validator_image:  none\n    vfn_config_path:  /path/to/config2.yaml\n    vfn_image:  none\n    create_vfns: false\n    stake_amount: 1000\n    count: 10\n"
        self.pangu_config_with_vfn = b"blueprints:\n \n nodebp:\n    nodes_persistent_volume_claim_size: size\n    validator_storage_class_name: standard\n    vfn_storage_class_name: standard\n    validator_config_path: /path/to/config.yaml\n    validator_image:  none\n    vfn_config_path:  /path/to/config2.yaml\n    vfn_image:  none\n    create_vfns: true\n    stake_amount: 1000\n    count: 10\n"

    def test_update_nodes_no_nodes(self) -> None:
        """Tests updating nodes"""
        expected_reads = {"pangu_node_config.yaml": self.pangu_config_no_vfn}
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        shell: SpyShell = SpyShell([])
        filesystem = SpyFilesystem({}, expected_reads)
        system_context = SystemContext(shell, filesystem, kubernetes)

        #
        # update nodes
        with self.assertRaises(Exception):
            update_nodes_main("default", "pangu_node_config.yaml", system_context)

    def test_update_nodes_no_vfn(self) -> None:
        expected_reads = {
            "pangu_node_config.yaml": self.pangu_config_no_vfn,
            "/path/to/config.yaml": b"",
        }
        expected_writes = {}
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        shell: SpyShell = SpyShell([])
        filesystem = SpyFilesystem(expected_writes, expected_reads)
        system_context = SystemContext(shell, filesystem, kubernetes)

        #
        # Add the nodes
        for i in range(1, 11):
            stateful_set = client.V1StatefulSet(
                metadata=client.V1ObjectMeta(name=f"nodebp-node-{i}-validator"),
                status=client.V1StatefulSetStatus(replicas=1),
            )
            kubernetes.create_resource(stateful_set, "default")
            configmap = client.V1ConfigMap(
                metadata=client.V1ObjectMeta(
                    name=f"nodebp-node-{i}-validator-configmap"
                )
            )
            kubernetes.create_resource(configmap, "default")

        update_nodes_main("default", "pangu_node_config.yaml", system_context)

        filesystem.assert_reads(self)
        filesystem.assert_writes(self)

    def test_update_nodes_with_vfn(self) -> None:
        expected_reads = {
            "pangu_node_config.yaml": self.pangu_config_with_vfn,
            "/path/to/config.yaml": b"",
            f"{util.TEMPLATE_DIRECTORY}/vfn.yaml": LocalFilesystem().read(
                "./pangu_lib/fixtures/vfn_1.yaml"
            ),
            "/tmp/vfn.yaml": LocalFilesystem().read("./pangu_lib/fixtures/vfn_1.yaml"),
        }
        expected_unlinks = ["/tmp/vfn.yaml"] * 10
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        shell: SpyShell = SpyShell([])
        filesystem = SpyFilesystem({}, expected_reads, expected_unlinks)
        system_context = SystemContext(shell, filesystem, kubernetes)

        #
        # Add the nodes
        for i in range(1, 11):
            stateful_set = client.V1StatefulSet(
                metadata=client.V1ObjectMeta(name=f"nodebp-node-{i}-validator"),
                status=client.V1StatefulSetStatus(replicas=1),
            )
            kubernetes.create_resource(stateful_set, "default")
            configmap = client.V1ConfigMap(
                metadata=client.V1ObjectMeta(
                    name=f"nodebp-node-{i}-validator-configmap"
                )
            )
            kubernetes.create_resource(configmap, "default")

            stateful_set = client.V1StatefulSet(
                metadata=client.V1ObjectMeta(name=f"nodebp-node-{i}-vfn"),
                status=client.V1StatefulSetStatus(replicas=1),
            )
            kubernetes.create_resource(stateful_set, "default")
            configmap = client.V1ConfigMap(
                metadata=client.V1ObjectMeta(name=f"nodebp-node-{i}-vfn-configmap")
            )
            kubernetes.create_resource(configmap, "default")

        update_nodes_main("default", "pangu_node_config.yaml", system_context)
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)
        filesystem.assert_unlinks(self)

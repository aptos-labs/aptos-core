from test_framework.kubernetes import SpyKubernetes
from kubernetes import client
import unittest
from kubernetes.client import ApiException  # type: ignore
from pangu_lib.node_commands.wipe_node import *
import pangu_lib.util as util


class node_wipe_node(unittest.TestCase):
    def test_wipe_node(self) -> None:
        """Tests wiping a node"""
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        expected_command = f"rm -rf {util.VELOR_DATA_DIR}/db/{util.LEDGER_DB_NAME} {util.VELOR_DATA_DIR}/db/{util.STATE_MERKLE_DB_NAME} {util.VELOR_DATA_DIR}/db/{util.STATE_SYNC_DB_NAME}"
        stateful_set = client.V1StatefulSet(
            metadata=client.V1ObjectMeta(name="node"),
            status=client.V1StatefulSetStatus(replicas=0),
        )
        kubernetes.create_resource(stateful_set, "default")
        pod = client.V1Pod(
            metadata=client.V1ObjectMeta(name="node-0"),
        )
        kubernetes.create_resource(pod, "default")

        #
        # Action
        wipe_node_main("default", "node", kubernetes)

        #
        # Assert kubernetes object
        self.assertEqual(
            expected_command, kubernetes.exec_commands["default"]["node-0"][0]
        )
        self.assertEqual(1, stateful_set.status.replicas)  # type: ignore

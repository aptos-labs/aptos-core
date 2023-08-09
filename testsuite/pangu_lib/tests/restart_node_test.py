from test_framework.kubernetes import SpyKubernetes
from kubernetes import client
import unittest
from kubernetes.client import ApiException  # type: ignore
from pangu_lib.node_commands.restart_node import *
from pangu_lib.node_commands.start_node import *
from pangu_lib.node_commands.stop_node import *


class node_restart_node(unittest.TestCase):
    def test_restart_running_node(self) -> None:
        """Tests starting a node"""
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        stateful_set = client.V1StatefulSet(
            metadata=client.V1ObjectMeta(name="restart-node"),
            status=client.V1StatefulSetStatus(replicas=1),
        )
        kubernetes.create_resource(stateful_set, "default")

        restart_node_main("default", "restart-node", kubernetes)

        #
        # Assertions
        if not stateful_set.status is None:
            self.assertEqual(1, stateful_set.status.replicas)
        else:
            self.fail("status is None")

    def test_restart_stopped_node(self) -> None:
        """Tests stopping a node"""
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        stateful_set = client.V1StatefulSet(
            metadata=client.V1ObjectMeta(name="restart-node"),
            status=client.V1StatefulSetStatus(replicas=0),
        )
        kubernetes.create_resource(stateful_set, "default")

        #
        # Actions
        stop_node_main("default", "restart-node", kubernetes)
        restart_node_main("default", "restart-node", kubernetes)

        #
        # Assertions
        if not stateful_set.status is None:
            self.assertEqual(1, stateful_set.status.replicas)
        else:
            self.fail("status is None")

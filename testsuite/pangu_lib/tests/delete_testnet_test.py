from test_framework.kubernetes import SpyKubernetes
from kubernetes import client
from pangu_lib.testnet_commands.delete_testnet import *
import unittest
from kubernetes.client import ApiException  # type: ignore


class testnet_tests_delete_testnet(unittest.TestCase):
    def test_delete_testnet(self) -> None:
        """Tests create_testnet on a CLI level"""
        kubernetes: SpyKubernetes = SpyKubernetes()
        with self.assertRaises(Exception):
            delete_testnet_main("wrong-name", False, kubernetes)
        with self.assertRaises(ApiException):  # type: ignore
            delete_testnet_main("pangu-doesnt-exist", False, kubernetes)
        kubernetes.namespaces["pangu-delete"] = client.V1Namespace()
        kubernetes.namespaced_resource_dictionary["pangu-delete"] = dict()
        try:
            delete_testnet_main("pangu-delete", False, kubernetes)
        except:
            self.fail("delete_testnet_main() raised exception unexpectedly!")

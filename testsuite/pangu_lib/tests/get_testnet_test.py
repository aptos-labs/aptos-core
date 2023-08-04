from test_framework.kubernetes import SpyKubernetes
from kubernetes import client
from pangu_lib.testnet_commands.get_testnet import get_testnet_main
import unittest
from kubernetes.client import ApiException  # type: ignore
from datetime import datetime, timezone


class testnet_tests_get_testnet(unittest.TestCase):
    def test_get_testnet(self) -> None:
        """Tests getting a testnet"""
        #
        # Init vars
        kubernetes: SpyKubernetes = SpyKubernetes()
        kubernetes.create_resource(
            client.V1Namespace(
                metadata=client.V1ObjectMeta(
                    name="pangu-get", creation_timestamp=datetime.now(timezone.utc)
                ),
                status=client.V1NamespaceStatus(phase="Active"),
            )
        )

        #
        # Assertions
        try:
            get_testnet_main("", "default", kubernetes)
        except:
            self.fail("get_testnet_main() raised exception unexpectedly!")
        try:
            get_testnet_main("pangu-get", "default", kubernetes)
        except:
            self.fail("get_testnet_main() raised exception unexpectedly!")
        try:
            get_testnet_main("", "json", kubernetes)
        except:
            self.fail("get_testnet_main() raised exception unexpectedly!")
        try:
            get_testnet_main("pangu-get", "json", kubernetes)
        except:
            self.fail("get_testnet_main() raised exception unexpectedly!")
        with self.assertRaises(Exception):
            get_testnet_main("doesnt-exist", "default", kubernetes)
        with self.assertRaises(Exception):
            get_testnet_main("doesnt-exist", "json", kubernetes)

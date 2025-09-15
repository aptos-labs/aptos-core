from test_framework.kubernetes import SpyKubernetes
from test_framework.filesystem import SpyFilesystem
from test_framework.shell import SpyShell
from pangu_lib.node_commands.add_pfn import *
import unittest
from kubernetes.client import ApiException  # type: ignore
from typing import Dict


class node_tests_add_pfn(unittest.TestCase):
    def test_add_pfn_dry(self) -> None:
        expected_reads: Dict[str, bytes] = {
            "pfn-config-path": open("./pangu_lib/fixtures/pfn_1.yaml", "rb").read(),
        }
        expected_writes: Dict[str, bytes] = {
            "workspace/pfn_dry_run": b"",
            "workspace/pfn_dry_run/pfn-name": b"",
            "workspace/pfn_dry_run/pfn-name/pfn_config_config_map.yaml": open(
                "./pangu_lib/fixtures/pfn_1_config_config_map.yaml", "rb"
            ).read(),
            "workspace/pfn_dry_run/pfn-name/pfn-service.yaml": open(
                "./pangu_lib/fixtures/pfn_1-service.yaml", "rb"
            ).read(),
            "workspace/pfn_dry_run/pfn-name/pfn-statefulset.yaml": open(
                "./pangu_lib/fixtures/pfn_1-statefulset.yaml", "rb"
            ).read(),
        }

        filesystem: SpyFilesystem = SpyFilesystem(expected_writes, expected_reads)

        system_args: SystemContext = SystemContext(
            SpyShell([]), filesystem, SpyKubernetes()
        )
        args: AddPFNArgs = AddPFNArgs(
            "testnet-name",
            "pfn-name",
            "pfn-config-path",
            "pfn-image",
            "workspace",
            "storage-class-name",
            "storage-size",
            "pfn-cpu",
            "pfn-memory",
        )

        try:
            add_pfn_main(args, system_args)
        except:
            self.fail("add_pfn_main() raised exception unexpectedly!")
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)

    def test_add_pfn_live(self) -> None:
        expected_reads: Dict[str, bytes] = {
            "pfn-config-path": open("./pangu_lib/fixtures/pfn_1.yaml", "rb").read(),
        }

        kubernetes: SpyKubernetes = SpyKubernetes()

        filesystem: SpyFilesystem = SpyFilesystem({}, expected_reads)

        system_args: SystemContext = SystemContext(SpyShell([]), filesystem, kubernetes)
        args: AddPFNArgs = AddPFNArgs(
            "default",
            "pfn-name",
            "pfn-config-path",
            "pfn-image",
            "",
            "storage-class-name",
            "storage-size",
            "pfn-cpu",
            "pfn-memory",
        )
        try:
            add_pfn_main(args, system_args)
        except:
            self.fail("add_pfn_main() raised exception unexpectedly!")
        self.assertEqual(4, len(kubernetes.namespaced_resource_dictionary["default"]))
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)

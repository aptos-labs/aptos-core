from click.testing import CliRunner, Result
from pangu_lib.testnet_commands import commands as testnet_commands
from typing import List
from test_framework.kubernetes import LiveKubernetes
from test_framework.shell import LocalShell, RunResult
from kubernetes import client
import unittest
import asyncio
import json
from test_framework.logging import log, init_logging, logging
import sys
import os

#
# INIT LOGGING
#
init_logging(log, level=logging.INFO, print_metadata=True)

#
# TODO The e2e tests are currently not being triggered in the CI/CD pipeline
#


class e2e_create_testnet_tests(unittest.TestCase):
    def setUp(self):
        self.runner = CliRunner()
        self.kubernetes = LiveKubernetes()
        self.shell = LocalShell()
        self.pod_time_out_seconds: int = 600
        os.environ["PYTHONUNBUFFERED"] = "1"

    def test_create_testnet(self) -> None:
        """Tests creating a testnet e2e"""
        #
        # Init vars
        bp_name: str = "nodebp"
        namespace: str = "e2e-test-create-testnet"
        expected_pod_names: set[str] = set()
        for index in range(1, 4):
            expected_pod_name_validator: str = f"{bp_name}-node-{index}-validator-0"
            expected_pod_names.add(expected_pod_name_validator)
            expected_pod_name_vfn: str = f"{bp_name}-node-{index}-vfn-0"
            expected_pod_names.add(expected_pod_name_vfn)
        random_validator_service_name: str = f"{bp_name}-node-1-validator"
        random_vfn_service_name: str = f"{bp_name}-node-1-vfn"
        #
        # Create testnet
        # Used to use --framework-path="./pangu_lib/template_testnet_files/framework.mrb"
        options: str = f'--name="{namespace}" --num-of-validators=3'
        result: Result = self.runner.invoke(testnet_commands.create, options)  # type: ignore

        #
        # Update namespace
        namespace: str = "pangu-e2e-test-create-testnet"

        # Assertions
        log.info("Asserting no exception has occured in the CLI command...")
        self.assertTrue(
            result.exception is None, f"Exception occurred: {result.exception}"
        )
        log.info("Asserting the exit code of the CLI command...")
        self.assertEqual(
            result.exit_code, 0, f"Exit code is {result.exit_code}. Expected: 0"
        )

        log.info("Asserting the expected pods exist...")
        pods: List[client.V1Pod] = self.kubernetes.get_pod_list(namespace).items
        self.assertEqual(6, len(pods))
        for pod in pods:
            pod_name: str = pod.metadata.name  # type: ignore
            self.assertTrue(pod_name in expected_pod_names)
        log.info("Asserted pods, now trying to port-forward...")
        #
        # Test ledger version
        asyncio.run(self._port_forwarding(namespace, random_validator_service_name))
        asyncio.run(self._port_forwarding(namespace, random_vfn_service_name))

    async def _port_forwarding(self, namespace: str, random_service_name: str):
        await asyncio.sleep(10)
        curl_command = [
            "kubectl",
            "get",
            "pods",
            f"{random_service_name}-0",
            "-o",
            "json",
            "-n",
            namespace,
        ]
        status = "Pending"
        curr: int = 0
        while status == "Pending":
            result = await self.shell.gen_run(curl_command, stream_output=True)
            output = result.output.decode("utf-8")
            try:
                service_info = json.loads(output)
            except Exception as e:
                log.error(e)
                curr += 10
                await asyncio.sleep(10)
                if curr > self.pod_time_out_seconds:
                    exit(1)
                continue
            status = service_info["status"]["phase"]
            if status == "Pending":
                log.info("Service is still pending. Waiting...")
                await asyncio.sleep(10)
            curr += 10
            if curr > self.pod_time_out_seconds:
                exit(1)
        log.info("Creating port-forwarding....")
        port_forwarding_command = [
            "kubectl",
            "port-forward",
            f"service/{random_service_name}",
            "8080:8080",
            "-n",
            namespace,
        ]
        process = await asyncio.create_subprocess_exec(
            *port_forwarding_command,
            stdout=sys.stdout,
            stderr=sys.stdout,
        )
        result = self.shell.gen_run(port_forwarding_command, stream_output=True)
        log.info("Created port-forwarding...")
        await asyncio.sleep(30)
        try:
            log.info("Trying to access the pod to see the ledger version...")
            curl_command = ["curl", "localhost:8080/v1"]
            result = await self.shell.gen_run(curl_command, stream_output=True)
            data = self._port_forwarding_result_parsing(result)
            ledger_version = int(data["ledger_version"])
            self.assertGreater(ledger_version, 0)
        finally:
            log.info("Done port-forwarding, cleaning up...")
            try:
                process.terminate()
                await process.wait()
                curl_command_elements = [
                    "kubectl",
                    "get",
                    "all",
                    "-n",
                    namespace,
                ]
                curl_command_logs = [
                    "kubectl",
                    "logs",
                    f"{random_service_name}-0",
                    "-n",
                    namespace,
                ]
                result_elements = await self.shell.gen_run(
                    curl_command_elements, stream_output=True
                )
                result_logs = await self.shell.gen_run(
                    curl_command_logs, stream_output=True
                )
                log.info("All the Kubectl elements are:")
                log.info("\n" + result_elements.output.decode("utf-8"))
                log.info("Kubectl logs are:")
                log.info("\n" + result_logs.output.decode("utf-8"))

            except ProcessLookupError:
                log.info("Port-forwarding process not found.")

    def _port_forwarding_result_parsing(self, result: RunResult):
        log.info("")
        log.info("--------------------------------------------------")
        log.info("The result output is:")
        log.info(result.output.decode("utf-8"))
        log.info("--------------------------------------------------")
        log.info("")
        lines = result.output.decode("utf-8").splitlines()
        output = lines[5]
        try:
            data = json.loads(output)
        except Exception as e:
            log.error(e)
            raise Exception(e)
        return data

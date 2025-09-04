import asyncio
from test_framework.kubernetes import Kubernetes
from test_framework.logging import log
from test_framework.shell import Shell, RunResult
from kubernetes import client
import json
from concurrent import futures
import pangu_lib.util as util
from multiprocessing import Value
from typing import Any  # , Callable
import tabulate


def healthcheck_main(
    testnet_name: str, endpoint_name: str, kubernetes: Kubernetes, shell: Shell
):
    """Runs a healthcheck on all nodes in a testnet

    Args:
        testnet_name (str): the namespace/testnet
        kubernetes (Kubernetes): kubernetes abstraction
        shell (Shell): shell abstraction

    """
    services = kubernetes.get_resources(client.V1Service, testnet_name)

    atomic_healthy_nodes: Any = Value("i", 0)
    atomic_unhealthy_nodes: Any = Value("i", 0)

    log.info("Starting healthcheck...")
    with futures.ThreadPoolExecutor() as executor:
        service_futures = []
        for service in services:
            service_name: str = service.metadata.name  # type: ignore
            future = executor.submit(
                _port_forwarding_wrapper, testnet_name, service_name, endpoint_name, atomic_healthy_nodes, atomic_unhealthy_nodes, shell  # type: ignore
            )
            service_futures.append(future)  # type: ignore
        futures.wait(service_futures)
        for future in service_futures:  # type: ignore
            if future.exception() is not None:  # type: ignore
                raise future.exception()  # type: ignore

    table_headers = ["TOTAL", "PASSED", "FAILED", "UNACCOUNTED"]
    table_data = [
        [
            len(services),
            atomic_healthy_nodes.value,
            atomic_unhealthy_nodes.value,
            len(services) - atomic_healthy_nodes.value - atomic_unhealthy_nodes.value,
        ]
    ]
    print("+---------+----------+----------+---------------+")
    print("|---------+-HEALTHCHECK SUMMARY-+---------------|")
    print(tabulate.tabulate(table_data, table_headers, tablefmt="grid"))


def _port_forwarding_wrapper(
    testnet_name: str,
    service_name: str,
    endpoint_name: str,
    atomic_healthy_nodes: Any,
    atomic_unhealthy_nodes: Any,
    shell: Shell,
):
    if endpoint_name == "ledger_info":
        asyncio.run(
            _port_forwarding_ledger(
                testnet_name,
                service_name,
                atomic_healthy_nodes,
                atomic_unhealthy_nodes,
                shell,
            )
        )
    elif endpoint_name == "healthy":
        asyncio.run(
            _port_forwarding_health(
                testnet_name,
                service_name,
                atomic_healthy_nodes,
                atomic_unhealthy_nodes,
                shell,
            )
        )
    else:
        raise Exception(f"Healthcheck format {endpoint_name} is not supported.")


async def _port_forwarding_health(
    namespace: str,
    service_name: str,
    atomic_healthy_nodes: Any,
    atomic_unhealthy_nodes: Any,
    shell: Shell,
):
    free_port: int = util.find_free_port()

    port_forwarding_command = [
        "kubectl",
        "port-forward",
        f"service/{service_name}",
        f"{free_port}:8080",
        "-n",
        namespace,
    ]
    process = await asyncio.create_subprocess_exec(
        *port_forwarding_command,
        # stdout=sys.stdout,
        stdout=asyncio.subprocess.DEVNULL,
    )
    await asyncio.sleep(5)
    try:
        curl_command = ["curl", f"localhost:{free_port}/v1/-/healthy?duration_secs=10"]
        result = await shell.gen_run(curl_command, stream_output=False)
        health_status = _parse_result("message", result)
        if health_status == "velor-node:ok":
            with atomic_healthy_nodes.get_lock():
                atomic_healthy_nodes.value += 1
        else:
            log.error(f"Healthcheck for service {service_name} failed the healthcheck.")
            with atomic_unhealthy_nodes.get_lock():
                atomic_unhealthy_nodes.value += 1
    finally:
        #
        # Done port-forwarding, cleaning up...
        try:
            process.terminate()
            await process.wait()
        except ProcessLookupError:
            log.error("Port-forwarding process not found.")


async def _port_forwarding_ledger(
    namespace: str,
    service_name: str,
    atomic_healthy_nodes: Any,
    atomic_unhealthy_nodes: Any,
    shell: Shell,
):
    free_port: int = util.find_free_port()
    port_forwarding_command = [
        "kubectl",
        "port-forward",
        f"service/{service_name}",
        f"{free_port}:8080",
        "-n",
        namespace,
    ]
    process = await asyncio.create_subprocess_exec(
        *port_forwarding_command,
        # stdout=sys.stdout,
        stdout=asyncio.subprocess.DEVNULL,
    )
    await asyncio.sleep(5)

    try:
        curl_command = ["curl", f"localhost:{free_port}/v1"]

        #
        # First, get the ledger version.
        result_1 = await shell.gen_run(curl_command, stream_output=False)
        ledger_version_1_str = _parse_result("ledger_version", result_1)
        if ledger_version_1_str == "Failed to parse result.":
            log.error(
                f"Healthcheck for node {service_name} failed the healthcheck because the node did not respond."
            )
            with atomic_unhealthy_nodes.get_lock():
                atomic_unhealthy_nodes.value += 1
            return

        ledger_version_1 = int(ledger_version_1_str)

        #
        # Then, wait 30 seconds.
        await asyncio.sleep(30)

        #
        # Then, get the ledger version again.
        result_2 = await shell.gen_run(curl_command, stream_output=False)
        ledger_version_2_str = _parse_result("ledger_version", result_2)
        if ledger_version_2_str == "Failed to parse result.":
            log.error(
                f"Healthcheck for node {service_name} failed the healthcheck because the node did not respond."
            )
            with atomic_unhealthy_nodes.get_lock():
                atomic_unhealthy_nodes.value += 1
            return
        ledger_version_2 = int(ledger_version_2_str)

        #
        # Then, compare the two ledger versions.
        if ledger_version_2 > ledger_version_1:
            with atomic_healthy_nodes.get_lock():
                atomic_healthy_nodes.value += 1
        else:
            log.error(
                f"Healthcheck for node {service_name} failed the healthcheck because the ledger version did not increase."
            )
            with atomic_unhealthy_nodes.get_lock():
                atomic_unhealthy_nodes.value += 1
    finally:
        #
        # Done port-forwarding, cleaning up...
        try:
            process.terminate()
            await process.wait()
        except ProcessLookupError:
            log.error("Port-forwarding process not found.")


def _parse_result(field: str, result: RunResult) -> str:
    try:
        lines = result.output.decode("utf-8").splitlines()
        output = lines[5]
        data = json.loads(output)
        field_data = data[field]
    except Exception:
        # log.error(e)
        field_data = "Failed to parse result."
    return field_data

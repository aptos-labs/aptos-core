from test_framework.kubernetes import Kubernetes
from test_framework.shell import Shell
from test_framework.logging import log
import asyncio
import webbrowser


def profile_node_main(
    testnet_name: str, node_name: str, kubernetes: Kubernetes, shell: Shell
):
    log.info("Opening Browser for profiling...")
    webbrowser.open("http://localhost:9101/profiling")
    log.info('Press "Ctrl + c" to end profiling....')
    asyncio.run(port_forwarding(testnet_name, node_name, shell))


async def port_forwarding(namespace: str, node_name: str, shell: Shell):
    port_forwarding_command = [
        "kubectl",
        "port-forward",
        f"service/{node_name}",
        "9101:9101",
        "-n",
        namespace,
    ]
    await shell.gen_run(port_forwarding_command)

import re
import click
from .create_testnet import create_testnet_main, CreateArgs, SystemContext
from .delete_testnet import delete_testnet_main
from .get_testnet import get_testnet_main
from .healthcheck import healthcheck_main
from .update_nodes import update_nodes_main
from .restart_nodes import restart_nodes_main
from .transaction_emitter import transaction_emitter_main
from test_framework.shell import LocalShell
from test_framework.filesystem import LocalFilesystem
from test_framework.kubernetes import LiveKubernetes
from typing import Optional, List
import random
import string
import os
import pwd
from test_framework.logging import log
import pangu_lib.util as util


@click.command(help="Create a testnet.")
@click.option(
    "--pangu-node-configs-path",
    help="Pass the path to the Pangu node configs (yaml). To see examples, see here: #TODO",
)
@click.option("--layout-path", help="Pass the path to the layout file (yaml).")
@click.option(
    "--framework-path",
    required=True,
    help="Pass in the path to the compiled move framework (head.mrb, or framework.mrb) file. To compile it, run: $ cargo run --locked --package velor-framework -- release",
)
@click.option(
    "--num-of-validators",
    default=10,
    help="Write the number of generic validators you would like to have in the testnet. Do not use this option if you are passing pangu node configs.",
)
@click.option(
    "--workspace",
    help="Pass the path to the folder you would like the genesis files to be generated (default is a temp folder).",
)
@click.option(
    "--velor-cli-path",
    default="velor",
    help="Pass the path to the velor CLI if it is not in your $PATH var.",
)
@click.option(
    "--dry-run",
    help="Pass in true if you would like to run genesis without deploying on K8S. All k8s YAML files will be dumped to the workspace",
    is_flag=True,
)
@click.option(
    "--name",
    help="Name for the testnet, default is a randomly generated name.",
)
def create(
    pangu_node_configs_path: Optional[str],
    num_of_validators: int,
    layout_path: Optional[str],
    workspace: Optional[str],
    framework_path: str,
    velor_cli_path: str,
    dry_run: bool,
    name: Optional[str],
):
    """this function allows you to create a testnet

    Args:
        pangu_node_configs_path (str): path to the pangu nod configs
        num_of_validators (int): number of validators (Do not use this option if you are passing pangu node configs)
        layout_path (str): path to the layout file
        workspace (str): path to the folder you would like the genesis files to be generated (default is a temp folder).
        framework_path (str): path to the compiled move framework
        velor_cli_path (str): path to velor cli
        dry_run (bool): whether to deploy to kubernetes, or save the deployment instructions to the workspace
        namespace (str): the namespace to create the testnet
    """

    testnet_name: str = (
        "".join(
            f"pangu-{pwd.getpwuid(os.getuid())[0]}-{''.join(random.choices(string.ascii_lowercase + string.digits, k=8))}"[
                :63
            ]
        )
        if name is None
        else "pangu-" + name
    )

    #
    # check if this testnet already exists
    try:
        get_testnet_main(testnet_name, "table", LiveKubernetes())
        raise Exception(
            f"Testnet {testnet_name} already exists. Please delete it before creating a new one."
        )
    except Exception as e:
        # The testnet does not exist...
        pass

    try:
        create_testnet_main(
            CreateArgs(
                pangu_node_configs_path=pangu_node_configs_path,
                num_of_validators=num_of_validators,
                layout_path=layout_path,
                workspace=workspace,
                framework_path=framework_path,
                velor_cli_path=velor_cli_path,
                dry_run=dry_run,
                name=testnet_name,
            ),
            SystemContext(LocalShell(), LocalFilesystem(), LiveKubernetes()),
        )
    except Exception as e:
        log.error(e, exc_info=True)
        #
        # Cleanup
        if not dry_run:
            try:
                delete_testnet_main(testnet_name, True, LiveKubernetes())
            except:
                pass
        log.error("Failed to create testnet!")
        raise Exception(e)


@click.command(help="Delete a testnet by name.")
@click.argument("testnet_name")
@click.option(
    "--wait_deletion",
    default=True,
    help="Pass false if you don't want to wait for the deletion of the namespace.",
)
def delete(testnet_name: str, wait_deletion: bool):
    """Deletes a testnet/namespace

    Args:
        testnet_name (str): the namespace/testnet name to be deleted
        wait_deletion (bool): whether to wait for the deletion or not
    """
    delete_testnet_main(testnet_name, wait_deletion, LiveKubernetes())


@click.command(help="Get a testnet by name.")
@click.argument("testnet_name", default="")
@click.option(
    "-o",
    "output_format",
    default="print",
    help="Pass for JSON output instead of table output.",
)
def get(testnet_name: str, output_format: str):
    """Get a testnet by name

    Args:
        testnet_name (str): testnet name
    """
    get_testnet_main(testnet_name, output_format, LiveKubernetes())


@click.command(help="Restart all nodes in a testnet by name.")
@click.argument("testnet_name")
def restart(testnet_name: str):
    """Restart a testnet by name."""
    restart_nodes_main(testnet_name, LiveKubernetes())


@click.command(help="Healthcheck a testnet by name.")
@click.argument("testnet_name")
@click.option(
    "-e",
    "--endpoint-name",
    default="ledger_info",
    help="Pass in the format of the healthcheck. Options are: ledger_info, and healthy. \
    Ledger check uses the v1 endpoint to check for ledger progress, and healthy_check uses the /v1/-/healthy endpoint to check health.",
)
def healthcheck(testnet_name: str, endpoint_name: str):
    """Healthcheck a testnet by name."""
    healthcheck_main(testnet_name, endpoint_name, LiveKubernetes(), LocalShell())


@click.command(help="Update a testnet by name.")
@click.argument("testnet_name")
@click.argument("pangu-node-configs-path")
def update(testnet_name: str, pangu_node_configs_path: str):
    """Update a testnet by name.

    Args:
        testnet_name (str): the testnet to update
        pangu_node_configs_path (str): path to the pangu node configs
    """
    update_nodes_main(
        testnet_name,
        pangu_node_configs_path,
        SystemContext(LocalShell(), LocalFilesystem(), LiveKubernetes()),
    )


@click.command(
    help="Create a transaction emitter for a testnet by name.",
    context_settings=dict(ignore_unknown_options=True),
)
@click.argument("testnet_name")
@click.option(
    "--dry-run",
    default=False,
    help="Pass in true if you would like to run genesis without deploying on K8S. All k8s YAML files will be dumped to the workspace",
)
@click.option("--workspace", default="/tmp", help="Pass the path to the workspace.")
@click.argument("args", nargs=-1, required=True)
def transaction_emitter(
    testnet_name: str, dry_run: bool, workspace: str, args: List[str]
):
    """Create a transaction emitter for a testnet by name.

    Args:
        testnet_name (str): the testnet to add a transaction emitter to
        dry_run (bool): whether to deploy to kubernetes, or save the deployment instructions to the workspace
        workspace (str): path to the folder you would like the genesis files to be generated (default is a temp folder).
    """
    transaction_emitter_main(
        testnet_name,
        dry_run,
        workspace,
        args,
        system_context=SystemContext(LocalShell(), LocalFilesystem(), LiveKubernetes()),
    )

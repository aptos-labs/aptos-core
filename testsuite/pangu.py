from pangu_lib.testnet_commands import commands as testnet_commands
from pangu_lib.node_commands import commands as node_commands
import click
from test_framework.logging import log, init_logging, logging
from typing import Any
import sys

#
# INIT LOGGING
#
init_logging(log, level=logging.INFO, print_metadata=True)


class CatchAllExceptions(click.Group):
    def __call__(self, *args: Any, **kwargs: Any):
        try:
            return self.main(*args, **kwargs)
        except Exception as exc:
            #
            # UNCOMMENT FOR MORE VERBOSE ERROR MESSAGES
            # log.error(exc, exc_info=True)
            click.echo("Exception: %s" % exc)
            sys.exit(1)


CONTEXT_SETTINGS = {
    "max_content_width": 140,
    "terminal_width": 140,
    "help_option_names": ["-h", "--help"],
}


@click.group("cli_wrapper", context_settings=CONTEXT_SETTINGS, cls=CatchAllExceptions)
def cli():
    pass


@cli.group()
def testnet():
    """The testnet subgroup:  Contains the functions to create, delete, get, healthcheck, and update a testnet"""
    pass


testnet.add_command(testnet_commands.create)
testnet.add_command(testnet_commands.delete)
testnet.add_command(testnet_commands.get)
testnet.add_command(testnet_commands.healthcheck)
testnet.add_command(testnet_commands.update)
testnet.add_command(testnet_commands.restart)


@cli.group()
def node():
    """The node subgroup: Contains the functions to update, restart, start, and stop a node"""
    pass


node.add_command(node_commands.restart)
node.add_command(node_commands.start)
node.add_command(node_commands.stop)
node.add_command(node_commands.wipe)
node.add_command(node_commands.profile)
node.add_command(node_commands.add_pfn)

if __name__ == "__main__":
    cli()

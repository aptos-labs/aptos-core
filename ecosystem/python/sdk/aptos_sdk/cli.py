# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import argparse
import asyncio
import sys
from typing import Dict, List, Tuple

from .account import Account
from .account_address import AccountAddress
from .aptos_cli_wrapper import AptosCLIWrapper
from .async_client import RestClient
from .ed25519 import PrivateKey
from .package_publisher import PackagePublisher


async def publish_package(
    package_dir: str,
    named_addresses: Dict[str, AccountAddress],
    signer: Account,
    rest_api: str,
):
    AptosCLIWrapper.compile_package(package_dir, named_addresses)

    rest_client = RestClient(rest_api)
    publisher = PackagePublisher(rest_client)
    await publisher.publish_package_in_path(signer, package_dir)


def key_value(indata: str) -> Tuple[str, AccountAddress]:
    split_indata = indata.split("=")
    if len(split_indata) != 2:
        raise ValueError("Invalid named-address, expected name=account address")
    name = split_indata[0]
    account_address = AccountAddress.from_str(split_indata[1])
    return (name, account_address)


async def main(args: List[str]):
    parser = argparse.ArgumentParser(description="Aptos Pyton CLI")
    parser.add_argument(
        "command", type=str, help="The command to execute", choices=["publish-package"]
    )
    parser.add_argument(
        "--account",
        help="The account to query or the signer of a transaction",
        type=AccountAddress.from_str,
    )
    parser.add_argument(
        "--named-address",
        help="A single literal address name paired to an account address, e.g., name=0x1",
        nargs="*",
        type=key_value,
    )
    parser.add_argument("--package-dir", help="The path to the Move package", type=str)
    parser.add_argument(
        "--private-key-path", help="The path to the signer's private key", type=str
    )
    parser.add_argument(
        "--rest-api",
        help="The REST API to send queries to, e.g., https://testnet.aptoslabs.com/v1",
        type=str,
    )
    parsed_args = parser.parse_args(args)

    if parsed_args.command == "publish-package":
        if parsed_args.account is None:
            parser.error("Missing required argument '--account'")
        if parsed_args.package_dir is None:
            parser.error("Missing required argument '--package-dir'")
        if parsed_args.rest_api is None:
            parser.error("Missing required argument '--rest-api'")

        if not AptosCLIWrapper.does_cli_exist():
            parser.error(
                "Missing Aptos CLI. Export its path to APTOS_CLI_PATH environmental variable."
            )

        if parsed_args.private_key_path is None:
            parser.error("Missing required argument '--private-key-path'")
        with open(parsed_args.private_key_path) as f:
            private_key = PrivateKey.from_str(f.read())

        account = Account(parsed_args.account, private_key)
        await publish_package(
            parsed_args.package_dir,
            dict(parsed_args.named_address),
            account,
            parsed_args.rest_api,
        )


if __name__ == "__main__":
    asyncio.run(main(sys.argv[1:]))

# Aptos Multisig Execution Expeditor (AMEE)

import argparse

def keyfile(args):
    print(f"Path to write to: {args.path}")

parser = argparse.ArgumentParser(
    prog='amee.py',
    description='Aptos Multisig Execution Expeditor (AMEE): A collection of ' \
                'tools designed to expedite multisig account execution.'
)

subparsers = parser.add_subparsers()

# Keygen command.
keygen_parser = subparsers.add_parser('keyfile')
keygen_parser.add_argument('path')
keygen_parser.set_defaults(func=keyfile)

args = parser.parse_args()

if hasattr(args, 'func'): # Invoke callback function as needed.
    args.func(args)
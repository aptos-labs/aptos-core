from forge import LocalShell

import re
from typing import Tuple
from pathlib import Path

import click



@click.group()
def main() -> None:
    pass


@main.command()
@click.argument("paths", nargs=-1)
def helm(paths: Tuple[str]) -> None:
    shell = LocalShell(True)

    error = False
    for path in paths:
        result = shell.run(["helm", "lint", path])
        for line in result.output.decode().splitlines():
            if line.startswith("[ERROR]"):
                match = re.match(r".ERROR. (?P<section>[^:]+?): (?P<error_type>.*) at [(](?P<filename>.*):(?P<line>\d+)[)]: (?P<message>.*)", line)
                if match:
                    fullpath = Path(path).parent / match.group("filename")
                    print("::error file={fullpath},line={line},col=1::{message}".format(fullpath=fullpath, **match.groupdict()))
                    error = True

    if error:
        raise SystemExit(1)



if __name__ == "__main__":
    main()
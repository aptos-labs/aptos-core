#!/usr/bin/python3
"""Boogie-compatible client for the controller's run-local proxy."""

from __future__ import annotations

import base64
import json
import os
import socket
import sys


def main() -> None:
    socket_path = os.environ.get("MOVE_INFERENCE_BOOGIE_PROXY")
    if not socket_path:
        raise SystemExit("boogie proxy: MOVE_INFERENCE_BOOGIE_PROXY is unset")
    request = {
        "arguments": sys.argv[1:],
        "cwd": os.getcwd(),
    }
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.connect(socket_path)
        connection.sendall(json.dumps(request, sort_keys=True).encode() + b"\n")
        response_bytes = b""
        while chunk := connection.recv(1024 * 1024):
            response_bytes += chunk
    response = json.loads(response_bytes)
    sys.stdout.buffer.write(base64.b64decode(response["stdout_base64"]))
    sys.stderr.buffer.write(base64.b64decode(response["stderr_base64"]))
    raise SystemExit(int(response["returncode"]))


if __name__ == "__main__":
    main()

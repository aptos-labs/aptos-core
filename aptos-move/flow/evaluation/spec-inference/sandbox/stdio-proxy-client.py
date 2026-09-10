#!/usr/bin/env python3
"""Bridge stdin/stdout to the controller's Unix-socket MCP process."""

from __future__ import annotations

import os
import selectors
import socket
import sys


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: stdio-proxy-client.py SOCKET")
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.connect(sys.argv[1])
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ, "stdin")
    selector.register(connection, selectors.EVENT_READ, "socket")
    while selector.get_map():
        for key, _ in selector.select():
            if key.data == "stdin":
                chunk = os.read(sys.stdin.fileno(), 65536)
                if chunk:
                    connection.sendall(chunk)
                else:
                    selector.unregister(sys.stdin.buffer)
                    connection.shutdown(socket.SHUT_WR)
            else:
                chunk = connection.recv(65536)
                if chunk:
                    os.write(sys.stdout.fileno(), chunk)
                else:
                    selector.unregister(connection)
    connection.close()


if __name__ == "__main__":
    main()

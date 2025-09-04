# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import logging
import socket

LOG = logging.getLogger(__name__)


def check_redis_is_running():
    LOG.info("Checking that Redis is running...")
    try:
        # Try to connect to Redis.
        connection = socket.create_connection(("127.0.0.1", 6379))
        connection.close()
    except Exception as e:
        raise RuntimeError(
            "Failed to connect to Redis, did you start a Redis server?"
        ) from e
    LOG.info("Redis is running!")

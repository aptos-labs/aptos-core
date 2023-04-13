# CLI logging.

import logging
import sys


# Decorator that injects the global "log" into a function scope.
def logger(func):
    func.__globals__["log"] = logging.getLogger("")
    return func


def init_logging(
    logger: logging.Logger,
    level: int = logging.INFO,
    print_metadata: bool = True,
) -> None:
    """Initialize logging."""
    logger.setLevel(level)
    sh = logging.StreamHandler(sys.stderr)
    if print_metadata:
        sh.setFormatter(
            logging.Formatter(
                "[%(asctime)s] %(levelname)s [%(filename)s.%(funcName)s:%(lineno)d] %(message)s",
                datefmt="%a, %d %b %Y %H:%M:%S",
            )
        )
    logger.addHandler(sh)

# CLI logging.

import logging
import sys

# just define a global logger
log = logging.getLogger("")


def init_logging(
    logger: logging.Logger,
    level: int = logging.INFO,
    print_metadata: bool = True,
) -> None:
    """Initialize logging for an application"""
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

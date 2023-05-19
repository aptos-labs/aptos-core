# A wrapper around the requests library to make simple HTTP requests.
# Like reqwest in Rust.

import requests
import sys
from requests import Response
import logging


class HttpClient:
    def get(self, url: str, headers: dict = None) -> requests.Response:
        raise NotImplementedError()


class SimpleHttpClient:
    logger: logging.Logger = logging.getLogger("")

    def get(self, url: str, headers: dict = None) -> requests.Response:
        return requests.get(url, headers=headers)

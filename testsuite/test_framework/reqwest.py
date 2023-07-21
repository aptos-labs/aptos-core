# A wrapper around the requests library to make simple HTTP requests.
# Like reqwest in Rust.

import requests
import sys
from requests import Response
import logging


class HttpClient:
    def get(self, url: str, headers: dict = {}) -> requests.Response:
        raise NotImplementedError()


class SimpleHttpClient(HttpClient):
    logger: logging.Logger = logging.getLogger("")

    def get(self, url: str, headers: dict = {}) -> requests.Response:
        return requests.get(url, headers=headers)

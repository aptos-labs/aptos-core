class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    def __init__(self, url: str) -> None:
        self.url = url

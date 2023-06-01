import importlib.metadata as metadata

# constants
PACKAGE_NAME = "aptos-sdk"


class Metadata:
    APTOS_HEADER = "x-aptos-client"

    @staticmethod
    def get_aptos_header_val():
        version = metadata.version(PACKAGE_NAME)
        return f"aptos-python-sdk/{version}"

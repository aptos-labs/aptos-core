#!/usr/bin/env python3
r"""
This script installs the Aptos CLI.

It will perform the following steps:
- Determine what platform (OS + arch) the script is being invoked from.
- Download the CLI.
- Put it in an appropriate location.

This was adapted from the install script for Poetry.
"""

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import sysconfig
import tempfile
import warnings
from contextlib import closing
from io import UnsupportedOperation
from pathlib import Path
from typing import Optional
from urllib.request import Request, urlopen, urlretrieve

try:
    from packaging.version import Version
except ImportError:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", category=DeprecationWarning)
        from distutils.version import StrictVersion as Version

SHELL = os.getenv("SHELL", "")
WINDOWS = sys.platform.startswith("win") or (sys.platform == "cli" and os.name == "nt")
MINGW = sysconfig.get_platform().startswith("mingw")
MACOS = sys.platform == "darwin"
SCRIPT = "aptos.exe" if WINDOWS else "aptos"
TEST_COMMAND = f"{SCRIPT} info"

X86_64 = ["x86_64", "amd64"]
SUPPORTED_ARCHITECTURES = {
    "macos": X86_64 + ["arm", "arm64", "aarch64"],
    "linux": X86_64,
    "windows": X86_64,
}

FOREGROUND_COLORS = {
    "black": 30,
    "red": 31,
    "green": 32,
    "yellow": 33,
    "blue": 34,
    "magenta": 35,
    "cyan": 36,
    "white": 37,
}

BACKGROUND_COLORS = {
    "black": 40,
    "red": 41,
    "green": 42,
    "yellow": 43,
    "blue": 44,
    "magenta": 45,
    "cyan": 46,
    "white": 47,
}

OPTIONS = {"bold": 1, "underscore": 4, "blink": 5, "reverse": 7, "conceal": 8}


def style(fg, bg, options):
    codes = []

    if fg:
        codes.append(FOREGROUND_COLORS[fg])

    if bg:
        codes.append(BACKGROUND_COLORS[bg])

    if options:
        if not isinstance(options, (list, tuple)):
            options = [options]

        for option in options:
            codes.append(OPTIONS[option])

    return "\033[{}m".format(";".join(map(str, codes)))


STYLES = {
    "info": style("cyan", None, None),
    "comment": style("yellow", None, None),
    "success": style("green", None, None),
    "error": style("red", None, None),
    "warning": style("yellow", None, None),
    "b": style(None, None, ("bold",)),
}


def is_decorated():
    if WINDOWS:
        return (
            os.getenv("ANSICON") is not None
            or "ON" == os.getenv("ConEmuANSI")
            or "xterm" == os.getenv("Term")
        )

    if not hasattr(sys.stdout, "fileno"):
        return False

    try:
        return os.isatty(sys.stdout.fileno())
    except UnsupportedOperation:
        return False


def is_interactive():
    if not hasattr(sys.stdin, "fileno"):
        return False

    try:
        return os.isatty(sys.stdin.fileno())
    except UnsupportedOperation:
        return False


def colorize(style, text):
    if not is_decorated():
        return text

    return f"{STYLES[style]}{text}\033[0m"


def string_to_bool(value):
    value = value.lower()

    return value in {"true", "1", "y", "yes"}


def bin_dir() -> Path:
    if WINDOWS and not MINGW:
        # ~ is %USERPROFILE% on Windows
        return Path("~/.aptoscli/bin").expanduser()
    else:
        return Path("~/.local/bin").expanduser()


PRE_MESSAGE = """Welcome to the {aptos} CLI installer!

This will download and install the latest version of the {aptos} CLI at this location:

{aptos_home_bin}
"""

POST_MESSAGE = """The {aptos} CLI ({version}) is installed now. Great!

You can test that everything is set up by executing this command:

{test_command}
"""

POST_MESSAGE_NOT_IN_PATH = """The {aptos} CLI ({version}) is installed now. Great!

To get started you need the {aptos} CLI's bin directory ({aptos_home_bin}) in your `PATH`
environment variable.
{configure_message}
Alternatively, you can call the {aptos} CLI explicitly with `{aptos_executable}`.

You can test that everything is set up by executing:

{test_command}
"""

POST_MESSAGE_CONFIGURE_UNIX = """
Add the following to your shell configuration file (e.g. .bashrc):

export PATH="{aptos_home_bin}:$PATH"

After this, restart your terminal.
"""

POST_MESSAGE_CONFIGURE_FISH = """
You can execute `set -U fish_user_paths {aptos_home_bin} $fish_user_paths`
"""

POST_MESSAGE_CONFIGURE_WINDOWS = """
Execute the following command to update your PATH:

setx PATH "%PATH%;{aptos_home_bin}"

After this, restart your terminal.
"""


class InstallationError(RuntimeError):
    def __init__(self, return_code: int = 0, log: Optional[str] = None):
        super().__init__()
        self.return_code = return_code
        self.log = log


class Installer:
    # The API returns the newest items first. Accordingly we expect the CLI release to
    # be in the last 100 releases (the max for a single page).
    METADATA_URL = (
        "https://api.github.com/repos/aptos-labs/aptos-core/releases?per_page=100"
    )

    def __init__(
        self,
        version: Optional[str] = None,
        force: bool = False,
        accept_all: bool = False,
        path: Optional[str] = None,
    ) -> None:
        self._version = version
        self._force = force
        self._accept_all = accept_all

        self._bin_dir = None
        self._release_info = None
        self._latest_release_info = None

    @property
    def bin_dir(self) -> Path:
        if not self._bin_dir:
            self._bin_dir = bin_dir()
        return self._bin_dir

    @property
    def bin_path(self):
        return self.bin_dir.joinpath(SCRIPT)

    @property
    def release_info(self):
        if not self._release_info:
            self._release_info = json.loads(self._get(self.METADATA_URL).decode())
        return self._release_info

    @property
    def latest_release_info(self):
        # Iterate through the releases and find the latest CLI release.
        for release in self.release_info:
            if release["tag_name"].startswith("aptos-cli-"):
                return release
        raise RuntimeError("Failed to find latest CLI release")

    def run(self) -> int:
        try:
            version, _current_version = self.get_version()
        except ValueError:
            return 1

        if version is None:
            return 0

        try:
            target = self.get_target()
        except:
            return 1

        if target is None:
            return 0

        self._write(colorize("info", "Determined target to be: {}".format(target)))
        self._write("")

        self.display_pre_message()

        try:
            self.install(version, target)
        except subprocess.CalledProcessError as e:
            raise InstallationError(return_code=e.returncode, log=e.output.decode())

        self._write("")
        self.display_post_message(version)

        return 0

    def install(self, version, target):
        self._install_comment(version, "Downloading...")

        self.bin_dir.mkdir(parents=True, exist_ok=True)
        if self.bin_path.exists():
            self.bin_path.unlink()

        url = self.build_binary_url(version, target)

        with tempfile.TemporaryDirectory() as tmpdirname:
            zip_file = os.path.join(tmpdirname, "aptos-cli.zip")
            urlretrieve(url, zip_file)
            # This assumes that the binary within the zip file is always
            # called `aptos` / `aptos.exe`.
            shutil.unpack_archive(zip_file, self.bin_dir)

        os.chmod(self.bin_path, 0o755)

        self._install_comment(version, "Done!")
        return 0

    def _install_comment(self, version: str, message: str):
        self._write(
            "Installing {} CLI ({}): {}".format(
                colorize("info", "Aptos"),
                colorize("b", version),
                colorize("comment", message),
            )
        )

    def build_binary_url(self, version: str, target: str) -> str:
        return f"https://github.com/aptos-labs/aptos-core/releases/download/aptos-cli-v{version}/aptos-cli-{version}-{target}.zip"

    def display_pre_message(self) -> None:
        kwargs = {
            "aptos": colorize("info", "Aptos"),
            "aptos_home_bin": colorize("comment", self.bin_dir),
        }
        self._write(PRE_MESSAGE.format(**kwargs))

    def display_post_message(self, version: str) -> None:
        if WINDOWS:
            return self.display_post_message_windows(version)

        if SHELL == "fish":
            return self.display_post_message_fish(version)

        return self.display_post_message_unix(version)

    def get_windows_path_var(self) -> Optional[str]:
        import winreg

        with winreg.ConnectRegistry(None, winreg.HKEY_CURRENT_USER) as root:
            with winreg.OpenKey(root, "Environment", 0, winreg.KEY_ALL_ACCESS) as key:
                path, _ = winreg.QueryValueEx(key, "PATH")

                return path

    def display_post_message_windows(self, version: str) -> None:
        path = self.get_windows_path_var()

        message = POST_MESSAGE_NOT_IN_PATH
        if path and str(self.bin_dir) in path:
            message = POST_MESSAGE

        self._write(
            message.format(
                aptos=colorize("info", "Aptos"),
                version=colorize("b", version),
                aptos_home_bin=colorize("comment", self.bin_dir),
                aptos_executable=colorize("b", self.bin_path),
                configure_message=POST_MESSAGE_CONFIGURE_WINDOWS.format(
                    aptos_home_bin=colorize("comment", self.bin_dir)
                ),
                test_command=colorize("b", TEST_COMMAND),
            )
        )

    def display_post_message_fish(self, version: str) -> None:
        fish_user_paths = subprocess.check_output(
            ["fish", "-c", "echo $fish_user_paths"]
        ).decode("utf-8")

        message = POST_MESSAGE_NOT_IN_PATH
        if fish_user_paths and str(self.bin_dir) in fish_user_paths:
            message = POST_MESSAGE

        self._write(
            message.format(
                aptos=colorize("info", "Aptos"),
                version=colorize("b", version),
                aptos_home_bin=colorize("comment", self.bin_dir),
                aptos_executable=colorize("b", self.bin_path),
                configure_message=POST_MESSAGE_CONFIGURE_FISH.format(
                    aptos_home_bin=colorize("comment", self.bin_dir)
                ),
                test_command=colorize("b", TEST_COMMAND),
            )
        )

    def display_post_message_unix(self, version: str) -> None:
        paths = os.getenv("PATH", "").split(":")

        message = POST_MESSAGE_NOT_IN_PATH
        if paths and str(self.bin_dir) in paths:
            message = POST_MESSAGE

        self._write(
            message.format(
                aptos=colorize("info", "Aptos"),
                version=colorize("b", version),
                aptos_home_bin=colorize("comment", self.bin_dir),
                aptos_executable=colorize("b", self.bin_path),
                configure_message=POST_MESSAGE_CONFIGURE_UNIX.format(
                    aptos_home_bin=colorize("comment", self.bin_dir)
                ),
                test_command=colorize("b", TEST_COMMAND),
            )
        )

    def get_version(self):
        latest_version = self.latest_release_info["tag_name"].split("-v")[-1]
        self._write(colorize("info", "Latest CLI release: {}".format(latest_version)))

        if self._force:
            return latest_version, None

        binary_path = self.bin_path
        try:
            out = subprocess.check_output(
                [binary_path, "--version"],
                universal_newlines=True,
            )
            current_version = current_version = out.split(" ")[-1].rstrip().lstrip()
        except Exception:
            current_version = None

        self._write(
            colorize("info", "Currently installed CLI: {}".format(current_version))
        )

        with warnings.catch_warnings():
            warnings.simplefilter("ignore", category=DeprecationWarning)
            if current_version and Version(current_version) >= Version(latest_version):
                self._write("")
                self._write(
                    f'The latest version ({colorize("b", latest_version)}) is already installed.'
                )

                return None, current_version

        return latest_version, current_version

    # Given the OS and CPU architecture, determine the "target" to download.
    def get_target(self):
        # We only look this up for validation, we only need the OS to figure out which
        # binary to download right now since we only build for x86_64 right now.
        arch = (platform.machine() or platform.processor()).lower()

        os = "windows" if WINDOWS else "macos" if MACOS else "linux"
        if not arch in SUPPORTED_ARCHITECTURES[os]:
            self._write(
                colorize(
                    "error",
                    f"The given OS ({os}) + CPU architecture ({arch}) is not supported.",
                )
            )
            return None

        if MACOS:
            return "MacOSX-x86_64"

        if WINDOWS:
            return "Windows-x86_64"

        # On Linux, we check what version of OpenSSL we're working with to figure out
        # which binary to download.
        try:
            out = subprocess.check_output(
                ["openssl", "version"],
                universal_newlines=True,
            )
            openssl_version = out.split(" ")[1].rstrip().lstrip()
        except Exception:
            self._write(
                colorize(
                    "warning",
                    "Could not determine OpenSSL version, assuming older version (1.x.x)",
                )
            )
            openssl_version = "1.0.0"

        if openssl_version.startswith("3."):
            return "Ubuntu-22.04-x86_64"

        return "Ubuntu-x86_64"

    def _write(self, line) -> None:
        sys.stdout.write(line + "\n")

    def _get(self, url):
        request = Request(url, headers={"User-Agent": "Aptos CLI Installer"})

        with closing(urlopen(request)) as r:
            return r.read()


def main():
    if sys.version_info.major < 3 or sys.version_info.minor < 6:
        sys.stdout.write(
            colorize("error", "This installer requires Python 3.6 or newer to run!")
        )
        # Return error code.
        return 1

    parser = argparse.ArgumentParser(
        description="Installs the latest version of the Aptos CLI"
    )
    parser.add_argument(
        "-f",
        "--force",
        help="Forcibly install on top of existing version",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "-y",
        "--yes",
        help="Accept all prompts",
        dest="accept_all",
        action="store_true",
        default=False,
    )

    args = parser.parse_args()

    installer = Installer(
        force=args.force,
        accept_all=args.accept_all or not is_interactive(),
    )

    try:
        return installer.run()
    except InstallationError as e:
        installer._write(colorize("error", "Aptos CLI installation failed."))

        if e.log is not None:
            import traceback

            _, path = tempfile.mkstemp(
                suffix=".log",
                prefix="aptos-cli-installer-error-",
                dir=str(Path.cwd()),
                text=True,
            )
            installer._write(colorize("error", f"See {path} for error logs."))
            text = f"{e.log}\nTraceback:\n\n{''.join(traceback.format_tb(e.__traceback__))}"
            Path(path).write_text(text)

        return e.return_code


if __name__ == "__main__":
    sys.exit(main())

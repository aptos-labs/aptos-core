from __future__ import annotations

import json
from forge import Filesystem


@dataclass
class GithubContext:
    

    @staticmethod
    def read(filesystem: Filesystem, path: str) -> GithubContext:
        contents = filesystem.read(path)
        return GithubContext(**json.loads(contents))
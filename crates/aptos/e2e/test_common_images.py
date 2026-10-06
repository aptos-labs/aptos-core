import sys
import unittest
from pathlib import Path

CI_DIR = next(parent / ".github" / "ci" for parent in Path(__file__).resolve().parents
              if (parent / ".github" / "ci").is_dir())
if str(CI_DIR) in sys.path:
    sys.path.remove(str(CI_DIR))
sys.path.insert(0, str(CI_DIR))
from tests.harness_support import ToolsImageContract, load_common

common = load_common("e2e_common_images_contract", Path(__file__).with_name("common.py"))
build_image_name = common.build_image_name


class ProtectedToolsImageTest(ToolsImageContract, unittest.TestCase):
    build_image = staticmethod(build_image_name)


if __name__ == "__main__":
    unittest.main()

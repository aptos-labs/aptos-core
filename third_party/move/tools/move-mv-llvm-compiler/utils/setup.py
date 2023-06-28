import os
import subprocess

def git_root():
  r = subprocess.getoutput("git rev-parse --show-toplevel")
  return r

def check_fmt_exe():
  pc="pre-commit-cargo-fmt"
  executable = os.access(os.path.join(git_root(), pc), os.X_OK)
  if not executable:
    print("{} is not an executable".format(pc))

check_fmt_exe()

def setup_precommit_hooks():
  subprocess.run(["git", "config", "--local", "core.hooksPath", "{}/utils".format(git_root())], check=True)


setup_precommit_hooks()

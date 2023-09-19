import subprocess


def invoke(cmd, **kwargs):
    proc = subprocess.run(cmd, shell=True, capture_output=True)
    if not kwargs.get('allow_non_zero', False):
        assert proc.returncode == 0
    return proc

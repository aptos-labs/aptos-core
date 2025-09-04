#!/usr/bin/env python3
#
# extract some useful parts from velor node config yaml
#
# 'api' get the host:port of the API server
# 'inspect' get the host:port of the inspection service endpoint
#
# usage:
#  python3 testsuite/node_info.py api /path/to/node
#  python3 testsuite/node_info.py inspect /path/to/node

import glob
import os
import sys

import yaml

keywords = {"api", "inspect"}


def try_yaml(fname, args):
    with open(fname, "rt") as fin:
        ob = yaml.full_load(fin)
    for arg in args:
        if arg == "api":
            print(ob["api"]["address"])
            return True
        elif arg == "inspect":
            print("localhost:{}".format(ob["inspection_service"]["port"]))
            return True
    return False


def try_dir(path, args):
    bad_paths = []
    for path in glob.glob(os.path.join(path, "*.yaml")):
        try:
            if try_yaml(path, args):
                return True
        except:
            pass
        bad_paths.append(path)
    sys.stderr.write("node config not found in: {}".format(", ".join(bad_paths)))
    return False


def main():
    args = []
    path = None
    for arg in sys.argv[1:]:
        if arg in keywords:
            args.append(arg)
        elif os.path.isdir(arg) or os.path.isfile(arg):
            path = arg
        else:
            sys.stderr.write("unknown arg {!r}".format(path))
            sys.exit(1)
    if path is None:
        sys.stderr.write("need some path to node data dir or config yaml")
        sys.exit(1)
    if not args:
        args = ["api"]
    if os.path.isdir(path):
        if not try_dir(path, args):
            sys.exit(1)
    elif os.path.isfile(path):
        if not try_yaml(path, args):
            sys.exit(1)
    else:
        sys.stderr.write("unknown arg {!r}".format(path))
        sys.exit(1)


if __name__ == "__main__":
    main()

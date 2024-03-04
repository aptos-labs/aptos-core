# Developer Guide

## Revela

Some CLI subcommands require [revela](https://github.com/verichains/revela) to work. We declare what version of revela to use in `revela/version.txt`. You'll notice this file looks like this:

```
v1.0.0-rc2
599fd222b861902f360c07b988bc488105f589076288dca31b93e2354ad1b884
```

The first line is a release tag, taken from the releases page: https://github.com/verichains/revela/tags. The second line is the sha256 of the tar.gz file for that release.

To update this file, run this script, where the argument is the release tag:
```
./revela/update.sh v1.0.0-rc2
```

It will pull the tar.gz file, get its sha256, and update the file.

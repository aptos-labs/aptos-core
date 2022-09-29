# Command line shorthands for accelerated developer workflows.
#
# This file contains assorted command line scripts for common developer
# tasks like building, testing, generating documentation, etc. For
# example, running a specific test conventionally requires the command
# line call:
#
# % aptos move test --filter <FILTER>
#
# Via the `tf` (test with filter) shorthand, this is compressed to:
#
# % s tf <FILTER>
#
# To use the below shorthands, simply add the following to your shell
# runtime configuration file, e.g. `~/.zshrc`:
#
# # Shorthands wrapper: pass all arguments to ./shorthands.sh
# s() {source shorthands.sh "$@"}
#
# Then you will be able to execute the below scripts via their specified
# shorthands.

# Build documentation.
if test $1 = d; then move build --doc

# Run all tests ("test all").
elif test $1 = ta; then aptos move test

# Run tests with using the provided filter string.
elif test $1 = tf; then aptos move test --filter $2

# Watch source code and build documentation if it changes. May require
# `brew install entr` beforehand.
elif test $1 = wd; then ls sources/*.move | entr move build --doc

# Watch source code and run a specific test it it changes, based on the
# provided filter string. May require `brew install entr` beforehand.
elif test $1 = wt; then ls sources/*.move | entr aptos move test --filter $2

else echo No such shorthand; fi
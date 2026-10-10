To run those examples, you need to have the 
[Aptos CLI installed](https://aptos.dev/build/cli).

You also need to have Claude Code installed, as well as the Move Flow 
Claude plugin. Currently, this need to be built from source as
described [here](../../../../../../aptos-move/flow/README.md). Also
see shell scripts in this directory to build everything.

Once you have `move-flow` in your path, you can
run `claude --plugin <path-to-plugin-dir>`.

Inside of Claude you can try 'infer specs', 'fix move code',
'prove', and other queries.


See the [Move on Aptos Book](https://aptos-labs.github.io/move-book)
for information about Move.

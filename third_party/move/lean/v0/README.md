# v0: deprecated reference packages

`move`, `move-model`, `transpiler`, and `scripts` are the original Leaner
Move stack: the source language with its `verify` engine over a shallow
semantics, the logical model of Move bytecode it compiled into, the
Move-to-Leaner transpiler, and their proof-cost tools. They are complete
for Move and are kept as reference for the solutions being rebuilt on the
leaner stack ([`../README.md`](../README.md)); nothing current depends on
them, they are not part of test runs or CI, and they receive no new
functionality or tests. Each is still an independent Lake package built
from its own directory.

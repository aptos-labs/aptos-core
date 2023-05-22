This module handles building (compiling) a global environment for a set of 
Move modules.

It can operate in two modes:

- *legacy mode*: it merges bytecode and the part of the Move sources which
  represent expression language constructs. The resulting model has full
  information of sources and bytecode.
- *compiler mode*: it fully analyzes the Move sources. In the resulting 
  model, bytecode related information is not available by default. However,
  bytecode can be attached in later phases using 
  `GlobalEnv::attach_compiled_module`.

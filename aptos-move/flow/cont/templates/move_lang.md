{% if once(name="move_lang") %}
## Move Language

Move is resource-oriented: abilities (`key`, `store`, `copy`, `drop`) determine
how values may be stored, duplicated, and discarded. Modules are published at
addresses; resources with `key` live in global storage.

- `entry fun` declares a transaction entry point; `#[view]` marks a read-only
  query.
- Use the package's established Move syntax and style. In Move 2 code, prefer
  `&T[addr]`, `&mut T[addr]`, and direct field access over legacy
  `borrow_global*`; do not add obsolete `acquires` annotations.
- Use named, documented abort constants rather than unexplained numeric codes.
- `///` is a declaration doc comment. Use `//` for comments inside functions.
- The edit hook checks and formats changed `.move` files. Treat its diagnostics
  as feedback, then confirm package status after completing the edit.


### Links

- [The Move Book](https://aptos-labs.github.io/move-book/)
- [Aptos Framework Book](https://aptos-labs.github.io/framework-book/)
{% endif %}

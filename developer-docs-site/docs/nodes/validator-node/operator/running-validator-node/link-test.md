---
title: "Link Test"
id: "link-test"
---

# Link Test

This is a temporary page for determining how to successfully link to files in both GitHub and our content system without error.

In short, when linking to absolute files (code, reference) In github.com, always use the fully qualified domain.
Else, use relative links.

> **Tip**: Each page can have an ID at the top that can override the path of the file. Best to check when linking or always use the recommended Markdown path (assuming we can get named anchors working).

[How Base Gas Works](../../../../concepts/base-gas.md) in relative link with Markdown extension should work in both

[How Base Gas Works](/concepts/base-gas) in absolute link with no Markdown extension works only in our content system

But what about named anchors?

How Base Gas Works [No Operation table](../../../../concepts/base-gas.md#no-operation) in relative link with Markdown extension should work in both

How Base Gas Works [No Operation table](/concepts/base-gas#no-operation) in absolute link with no Markdown extension works only in our content system
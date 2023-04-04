---
title: "Follow Aptos Style"
slug: "aptos-style"
---

# Follow Aptos Writing Style

When making [site updates](./site-updates.md), Aptos recommends adhering to this writing and formatting style guide for consistency with the rest of Aptos.dev, as well as accessibility directly in GitHub.com and source code editors.

## Hold contributions to high standards

All doc updates should be thorough and tested. This includes external contributions from the community.

So when reviewing changes, do not merge them in unless all feedback has been addressed.

## Single source in Markdown

There should be one external upstream source of truth for Aptos development. And we aim for that to be Aptos.dev. Edit away in [Markdown](https://www.markdownguide.org/basic-syntax/) format using our instructions for making [site updates](./site-updates.md).

Note, you can easily convert Google Docs to Markdown format using the [Docs to Markdown](https://workspace.google.com/marketplace/app/docs_to_markdown/700168918607) add-on.

## Link from product to docs

Whether you work on an external product or an internal tool, your work likely has an interface. From it, you should link to your user docs, along with bug queues and contact information.

## Peer review docs

Your users should not be the first people to use your documentation. Have your peers review your docs just as they review your code. Walk through the flow. If they cannot, your users can't either.

## Form links properly

When linking to absolute files (code, reference) not on Aptos.dev, always use the fully qualified domain. Else, use relative links. Always include the file extension (`.md` for Markdown).

Correct:

```markdown
[How Base Gas Works](../../../../concepts/base-gas.md)
```

Incorrect:

```markdown
[How Base Gas Works](/concepts/base-gas)
```

The second example will work in [Aptos.dev](http://Aptos.dev) but not when navigating the docs via [GitHub.com](http://GitHub.com) or in source viewer/editor. For links to files in the same directory, include the leading `./` like so:

```markdown
[proofs](./txns-states.md#proofs)
```

## Use permanent links to code

When linking to code files in GitHub, use a [permanent link](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-a-permanent-link-to-a-code-snippet) to the relative line or set of lines.

## Link check your pages

It never hurts to run a link check against your pages or entire site. Here are some freely available and useful tools for **public** site checking:

  * https://validator.w3.org/checklink
  * https://www.drlinkcheck.com/

Set recursion depth accordingly to delve into sub-links.

## Add images to `static` directory

Place all images in the [`developer-docs-site/static/img`](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/img) directory and use relative links to include them. See the image paths in [Set up a React app](../tutorials/build-e2e-dapp/2-set-up-react-app.md) for examples.

## Redirect moved pages

Avoid losing users by adding redirects for moved and renamed [Aptos.dev](http://Aptos.dev) pages in:
https://github.com/aptos-labs/aptos-core/blob/main/developer-docs-site/docusaurus.config.js

## Name files succinctly

Use short, detailed names with no spaces:
* hyphenate rather than underscore

* be descriptive
* use noun (topic) first, with verb optional: ex. accounts.md, site-updates.md

## Use active tense

Avoid passive tense and gerunds when possible:

- Good - Use Aptos API
- Not-so-good - Using Aptos API
- Needs improvement - Aptos API Use

## Employ direct style and tone

- Address the user directly. Use "you" instead of "user" or "they".
- Avoid writing the way you speak, i.e., avoid using contractions, jokes or using colloquial content.
    
    💡 **Example**:
    
    - **Preferred**: “it will” or “we will” or “it would”.
    - **Avoid**: “it’ll” or “we’ll” or “it’d”.
    
- Use the active voice.
    
    💡 **Example**:
    
    - **Preferred**: Fork and clone the Aptos repo.
    - **Avoid**: The Aptos repo should be cloned.
    - **Preferred**: Copy the `Config path` information from the terminal.
    - **Avoid**: The `Config path` information should be copied from the terminal.
    
- Avoid hypothetical future "would". Instead, write in present tense.
    
    💡 **Example**:
    
    - **Preferred**: "The compiler sends".
    - **Avoid**: “The compiler would then send”.

## Ensure readability

- Break up walls of text (long passages of text) into smaller chunks for easier reading.
- Use lists. When you use lists, keep each item as distinct as you can from another item.
- Provide context. Your readers can be beginner developers or experts in specialized fields. They may not know what you are talking about without any context.
- Use shorter sentences (26 words or less) They are easier to understand (and translate).
- Define acronyms and abbreviations at the first usage in every topic.
- Keep in mind our documentation is written in US English, but the audience will include people for whom English is not their primary language.
- Avoid culturally specific references, humor, names.
- Write dates and times in unambiguous and clear ways using the [international standard](https://en.wikipedia.org/wiki/Date_format_by_country). Write "27 November 2020" instead of either "11/27/2020" or "27/11/2020" or "November 27, 2020".
- Avoid negative sentence construction.
    
    💡 **Example**:
    
    - **Preferred**: It is common.
    - **Avoid**: It is not uncommon.
    
    Yes there is a subtle difference between the two, but for technical writing this simplification works better.
    
- Avoid directional language (below, left) in procedural documentation, **unless** you are pointing to an item that is immediately next to it.
- Be consistent in capitalization and punctuation.
- Avoid the `&` character in the descriptive text. Use the English word "and".

## Avoid foreshadowing

- Do not refer to future features or products.
- Avoid making excessive or unsupported claims about future enhancements.

## Use proper casing

Use title case for page titles and sentence case for section headers. Ex:

- Page title - Integrate Aptos with Your Platform
- Section title - Choose a network

Of course, capitalize [proper nouns](https://www.scribbr.com/nouns-and-pronouns/proper-nouns/), such as “Aptos” in “Accounts on Aptos”

## Write clear titles and headings

- Document titles and section headings should:
    - Explicitly state the purpose of the section.
    - Be a call to action, or intention.

This approach makes it easier for the reader to get her specific development task done.

💡 **Examples**

- **Preferred**: Running a fullnode (section heading)
- **Avoid**: FullNode running fundamentals (title is not purpose-driven)
- **Preferred**: Creating your first Move module
- **Avoid**: Move module

**Document titles (h1)**

- Use title case. For example: "Running a Model"

A document title is the main title of a document page. A document has only one document title.

💡 **Example**: "Writing Style Guide" at the beginning of this page. The document title also appears at the top level in the navigation bar, so it must be short, preferably four to five words or less.


**Section headings within a document (h2, h3, h4, h5)**

- Use sentence case. **For example**: "Verify initial synchronization"

A section heading is the title for an individual section within a document page. 

💡 **Example**: "Titles and headings" at the top of this section. A document page can have multiple sections, and hence multiple section headings.

- Use a heading hierarchy. Do not skip levels of the heading hierarchy. **For example**, put h3 only under h2.
- To change the visual formatting of a heading, use CSS instead of using a heading level that does not fit the hierarchy.
- Do not keep blank headings or headings with no associated content.
- Avoid using question mark in document titles and section headings.
    
    💡 **Example**:
    
    - **Preferred**: How it works
    - **Avoid**: How it works?
    
- Avoid using emphasis or italics in document titles or section headings.
- Avoid joining words using a slash.
    
    💡 **Example**:
    
    - **Preferred**: Execute on your macOS or Linux system
    - **Avoid**: Execute on your macOS/Linux system

## Avoid duplication

We face too many challenges to tackle the same one from scratch again or split our efforts into silos. We must collaborate to make best use of our diverse and growing skillset.

Search and navigate across this site to see if an existing document already serves your purpose and garners an update before starting anew. As with code, [don't repeat yourself](https://www.wikipedia.org/wiki/Don%27t_repeat_yourself).
    
## Use these Aptos words and phrases consistently

The below table lists the correct usage of Aptos words and phrases. 

| Recommended way to use in mid-sentence  | Avoid these forms |
| --- | --- |
| First letter uppercase if appearing at the start of a sentence. |  |
| fullnode (FN) | FullNode, Fullnode |
| validator or validator node (VN) | Validator Node, ValidatorNode |
| validator fullnode (VFN) | Validator FullNode or ValidatorFullNode |
| public fullnode | Public FullNode |
| Aptos blockchain | Aptos Blockchain |
| Move module | Move Module |
| Move resource | Move Resource |
| Aptos framework | Aptos Framework |
| Faucet | faucet |
| mempool | Mempool |
| bytecode | bytecodes |
| MoveVM | Move VM |
| REST service | REST Service |
| upgradeable | upgradable |

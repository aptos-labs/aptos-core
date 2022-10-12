## Workflow versions
Workflow versions are pinned using this tool: https://github.com/mheap/pin-github-action. We can use this same tool to (carefully) update the pinned versions if the need arises.
```
find . -type f -name "*.yaml" | xargs -I@ pin-github-action @
find . -type f -name "*.yaml" | xargs -I@ prettier -w @
```
After this you have to go fix some formatting and remove some erroneous `null`s that the tool added: https://github.com/mheap/pin-github-action/issues/111.


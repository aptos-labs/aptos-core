# Move Documentation

Two mdbook sites live here, both published from this repo to GitHub Pages
under the `aptos-labs.github.io` org site.

| Source                   | Deployed URL                                       | How to deploy                  |
| ------------------------ | -------------------------------------------------- | ------------------------------ |
| [`book/`](./book)         | <https://aptos-labs.github.io/move-book/>          | `./book/deploy.sh`             |
| [`framework-book/`](./framework-book) | <https://aptos-labs.github.io/framework-book/> | `./framework-book/deploy.sh`   |

`book/` is the hand-authored Move language reference. `framework-book/`
is generated automatically from the framework source by
`framework-book/build.sh` (which the deploy script invokes).

## Deploying

By default both deploy scripts **refuse to publish from a HEAD that
hasn't been merged into the canonical `main` branch**. Otherwise the
commit hash baked into each rendered page would point at a private
commit that no one else can resolve.

The scripts find the canonical remote automatically — `upstream` if you
configured it (typical when working from a fork), otherwise `origin`
(when you cloned `aptos-labs/aptos-core` directly). Override with
`UPSTREAM_REMOTE=<name>` if neither convention matches your setup.

Standard flow — land first, deploy second:

1. Land your PR.
2. `git fetch <remote> && git checkout <remote>/main` (substitute
   `upstream` or `origin` depending on your setup).
3. Run the relevant `deploy.sh`.

To override the merged-branch requirement (e.g. for local iteration
before a PR has merged), pass `--debug`:

```sh
./book/deploy.sh --debug
./framework-book/deploy.sh --debug
```

The build-stamp footer of every page is then labelled `debug build`, so
a deploy from an unmerged commit is visibly distinct from a normal one.

See each script's `--help` for the full list of options (`SUBPATH`,
`DRY_RUN`, `PAGES_REPO`, etc.).

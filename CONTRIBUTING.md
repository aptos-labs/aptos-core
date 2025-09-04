---
id: contributing
title: Contributing to Velor Core
---

# Contributing

Our goal is to make contributing to Velor Core easy and transparent. See [Velor Community](https://velor.dev/community) for full details. This page describes [our development process](#our-development-process).

## Velor Core

To contribute to the Velor Core implementation, first start with the proper development copy.
You may want to use the GitHub interface to fork [velor-core](https://github.com/velor-chain/velor-core) and check out your fork.
For development environment setup and first build, see [Building Velor From Source](https://velor.dev/guides/building-from-source)

## Our Development Process

### Code Style, Hints, and Testing

Refer to our Coding Guidelines for the [Move](https://velor.dev/move/book/coding-conventions/) and [Rust](./RUST_CODING_STYLE.md) programming languages for detailed guidance about how to contribute to the project.

Also, please ensure you follow our [Secure Coding Guidelines](./RUST_SECURE_CODING.md) to contribute to Velor securely. 


### Documentation

Velor Core's developer website is also open source (the code can be found in this [repository](https://github.com/velor-chain/developer-docs).  It is built using [Docusaurus](https://docusaurus.io/):

If you know Markdown, you can already contribute!

## Developer Workflow

Changes to the project are proposed through pull requests. The general pull request workflow is as follows:

* If you have added code that should be tested, add unit tests.
* If you have changed APIs, update the documentation. Make sure the documentation builds.
* Ensure all tests and lints pass on each and every commit that is part of your pull request using `./scripts/rust_lint.sh`.
* Submit your pull request.

## Authoring Clean Commits

### Logically Separate Commits

Commits should be [atomic](https://en.wikipedia.org/wiki/Atomic_commit#Atomic_commit_convention) and broken down into logically separate changes. Diffs should also be made easy for reviewers to read and review so formatting fixes or code moves should not be included in commits with actual code changes.

### Meaningful Commit Messages

Commit messages are important and incredibly helpful for others when they dig through the commit history in order to understand why a particular change was made and what problem it was intending to solve. For this reason commit messages should be well written and conform with the following format:

All commit messages should begin with a single short (50 character max) line summarizing the change and should skip the full stop. This is the title of the commit. It is also preferred that this summary be prefixed with "[area]" where the area is an identifier for the general area of the code being modified, e.g.

```
* [ci] enforce whitelist of nightly features
* [language] removing VerificationPass trait
```

A non-exhaustive list of some other areas include:
* consensus
* mempool
* network
* storage
* execution

Following the commit title (unless it alone is self-explanatory), there should be a single blank line followed by the commit body which includes more detailed, explanatory text as separate paragraph(s). It is recommended that the commit body be wrapped at 72 characters so that Git has plenty of room to indent the text while still keeping everything under 80 characters overall.

The commit body should provide a meaningful commit message, which:
* Explains the problem the change tries to solve, i.e. what is wrong with the current code without the change.
* Justifies the way the change solves the problem, i.e. why the result with the change is better.
* Alternative solutions considered but discarded, if any.

### References in Commit Messages

If you want to reference a previous commit in the history of the project, use the format "abbreviated sha1 (subject, date)", with the subject enclosed in a pair of double-quotes, like this:

```bash
Commit 895b53510 ("[consensus] remove slice_patterns feature", 2019-07-18) noticed that ...
```

This invocation of `git show` can be used to obtain this format:

```bash
git show -s --date=short --pretty='format:%h ("%s", %ad)' <commit>
```

If a commit references an issue please add a reference to the body of your commit message, e.g. `issue #1234` or `fixes #456`. Using keywords like `fixes`, `resolves`, or `closes` will cause the corresponding issue to be closed when the pull request is merged.

Avoid adding any `@` mentions to commit messages, instead add them to the PR cover letter.

## Responding to Reviewer Feedback

During the review process a reviewer may ask you to make changes to your pull request. If a particular commit needs to be changed, that commit should be amended directly. Changes in response to a review *should not* be made in separate commits on top of your PR unless it logically makes sense to have separate, distinct commits for those changes. This helps keep the commit history clean.

If your pull request is out-of-date and needs to be updated because `main` has advanced, you should rebase your branch on top of the latest main by doing the following:

```bash
git fetch upstream
git checkout topic
git rebase -i upstream/main
```

You *should not* update your branch by merging the latest main into your branch. Merge commits included in PRs tend to make it more difficult for the reviewer to understand the change being made, especially if the merge wasn't clean and needed conflicts to be resolved. As such, PRs with merge commits will be rejected.

## Bisect-able History

It is important that the project history is bisect-able so that when regressions are identified we can easily use `git bisect` to be able to pin-point the exact commit which introduced the regression. This requires that every commit is able to be built and passes all lints and tests. So if your pull request includes multiple commits be sure that each and every commit is able to be built and passes all checks performed by CI.

## Issues

Velor Core uses [GitHub issues](https://github.com/velor-chain/velor-core/issues) to track bugs. Please include necessary information and instructions to reproduce your issue.

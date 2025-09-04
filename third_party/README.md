# Third Party Crates

This directory contains synchronized copies of selected external repositories. Those repos are mirrored in the velor-core repo because they are core to the security of Velor -- and control over the source should therefore be isolated. They are also mirrored to allow atomic changes across system boundaries. For example, the [Move repository](https://github.com/move-language/move) has a copy in this tree, so we can apply changes simultaneously to Move and Velor.

In general:

- Code can be submitted in this directory using an velor-core wide PR. 
- (_For admins only_) Periodically, changes in this directory are pushed upstream or pulled from upstream, using the [copybara](https://github.com/google/copybara) tool. Those pushes will preserve the commit metadata (author, description) and copy it from one repo to another. 

## Guidelines for Developers

You should be able to happily code away and submit to this tree. Synchronization with the upstream repos is handled (for now) by someone else. However, there are a few things to keep in mind for clean code in this tree:

- Respect abstraction boundaries. The repos mirrored in this tree are independent, standalone projects and should stay like this. 
- Do not create path-dependencies from `third_party` to any crates _outside this tree_. As a rule of thumb, consider that the code in each of the mirrored repos _must compile and test independently_ when synced back to upstream. (Over time, we will likely create a nightly job to ensure that this is the case.)
- Try to partition changes, so you have independently documented commits for changes in this tree compared to changes outside. Those commits can still be part of one PR. When the code is synced upstream this will enable a more coherent commit history. _Do not squash on merge_ to preserve those commits.
 

## Guidelines for Admins

### Copybara Approach

It is recommended to take a look at the [copybara tutorial](https://blog.kubesimplify.com/moving-code-between-git-repositories-with-copybara) to familiarize yourself with the basic concepts. In a nutshell copybara works as follows: 

- assume we have repos A and B and syncing code from A to B 
- the tool knows a commit hash H from A s.t. H is the last state which synced from A to B. It either finds this hash in the commit history of B via a tag like `GitOrigin-RevId: <H>` in the commit messages, or it is provided via the flag `--baseline-for-merge-import=H`.
- The tool furthermore knows a commit hash M from B which is the parent for the change in B. This is provided with the flag `--change-request-parent=M`.
- The tool then computes all the commits needed to bring B from the last state H to the newest state

### Pushing

Pushing to [the `velor-main` branch in the Move repo](https://github.com/move-language/move/tree/velor-main) should be only be performed if this branch is not ahead of `third_party/move`, that are no outstanding changes which have not been pulled into velor-core. This generally simplifies pushing, and will eventually allow us to fully automate it via a nightly job. 

Currently, pushing has to be done manually. Below, substitute `/Users/wrwg/move` by your path to a local git repo of the Move language:

```shell
copybara copy.bara.sky push_move --output-root=/tmp --git-destination-url=file:///Users/wrwg/move
```

This will create a branch `to_move` which can then be submitted to the upstream Move.

### Pulling

Code which is pulled from the Move repo might be derived from an older version than the current `main` of velor-core.

```
        velor-main
       /          \
      / pull       \
      |             \ external contribution
      | PRs in
      | third_party
```

For this reason, pulling is a bit more complex right now and requires some extra work. 

1. Checkout velor-core to the commit of the last pull from the Move repo, into a branch `from_move` 
   ```shell
   git checkout <hash>
   git switch -c from_move
   ```
2. Run the following command, where `/Users/wrwg/velor-core` is replaced by our path to the velor-core repo:
   ```shell
   copybara copy.bara.sky pull_move --output-root=/tmp --git-destination-url=file:///Users/wrwg/velor-core
   ```
   This will add a series of commits to the branch `from_move`
3. Rebase `from_move` onto the current `main branch`
   ```shell
   git rebase main
   ```
   Any conflicts are now those of the external contributions relative to the progress in `third_party` and for you to resolve. After that, submit as a PR.


### Installing Copybara

Copybara must be build from source. 

#### MacOS

We first need Java. If its not yet in your path (`java` should show), you can install the openjdk with relative little hassle:

```shell
brew update
brew install java
```

The last step should print out instructions how to update the PATH so `java` is found.

We also need bazel:

```shell
brew install bazel
```

Finally we can clone the copybara repo and compile the program:

```shell
git clone https://github.com/google/copybara.git
cd copybara
bazel build //java/com/google/copybara
alias copybara="$PWD/bazel-bin/java/com/google/copybara/copybara"
```

#### Linux

TBD

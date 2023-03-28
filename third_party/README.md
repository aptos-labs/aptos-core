# Third Party Crates

This directory contains synchronized copies of selected external repositories. Those repos are mirrored in the aptos-core repo because they are core to the security of Aptos -- and control over the source should therefore be isolated. They are also mirrored to allow atomic changes across system boundaries. For example, the [Move repository](https://github.com/move-language/move) has a copy in this tree, so we can apply changes simultaneously to Move and Aptos.

In general:

- Code can be submitted in this directory using an aptos-core wide PR. 
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

TODO: flash out this description as we improve understanding

### Source of Truth: Aptos Core

Copybara can be used bidirectional using push and pull workflows as described below. However, those workflows will by default override changes made since the destinations has evolved independently. If there is no clear source of truth, the PRs created by the workflows below *must be rebased on the destination*, resolving potential merge conflicts manually.

This problem does not arise if a unique source-of-truth is maintained, i.e. PRs are submitted to only one repo. This is `aptos-core`. However, occasionally we need to import back from `move` into `aptos-core`, in which case the PRs need to be manually rebased.

### Pushing

In order to push to the Move repo, use:

```shell
copybara copy.bar.sky push_move --output-root=/tmp
```

This will create a draft PR in move (in the fixed branch `from_aptos`) with the needed changes. The PR should be massaged and send out for regular review.


### Pulling

Assuming `copybara` is available from the command line, to pull from the Move repo (for example), use:


```shell
copybara copy.bar.sky pull_move --output-root=/tmp 
```

This will create a draft PR in aptos_core (in the fixed branch `from_move`) with the needed changes. The PR should be massaged and send out for regular review.


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

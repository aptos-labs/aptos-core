// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/**
The high level parallel execution logic is implemented in 'executor.rs'. The
input of parallel executor is a block of transactions, containing a sequence
of n transactions tx_1, tx_2, ..., tx_n (this defines the preset serialization
order tx_1< tx_2< ...<tx_n).

Each transaction might be executed several times and we refer to the i-th
execution as incarnation i of a transaction. We say that an incarnation is
aborted when the system decides that a subsequent re-execution with an incremented
incarnation number is needed. A version is a pair of a transaction index and
an incarnation number. To support reads and writes by transactions that may
execute concurrently, parallel execution maintains an in-memory multi-version
data structure that separately stores for each memory location the latest value
written per transaction, along with the associated transaction version.
This data structure is implemented in: '../../mvhashmap/src/lib.rs'.
When transaction tx reads a memory location, it obtains from the multi-version
data-structure the value written to this location by the highest transaction
that appears before tx in the preset serialization order, along with the
associated version. For example, transaction tx_5 can read a value written
by transaction tx_3 even if transaction tx_6 has written to same location.
If no smaller transaction has written to a location, then the read
(e.g. all reads by tx_1) is resolved from storage based on the state before
the block execution.

For each incarnation, parallel execution maintains a write-set and a read-set
in 'txn_last_input_output.rs'. The read-set contains the memory locations that
are read during the incarnation, and the corresponding versions. The write-set
describes the updates made by the incarnation as (memory location, value) pairs.
The write-set of the incarnation is applied to shared memory (the multi-version
data-structure) at the end of execution. After an incarnation executes it needs
to pass validation. The validation re-reads the read-set and compares the
observed versions. Intuitively, a successful validation implies that writes
applied by the incarnation are still up-to-date, while a failed validation implies
that the incarnation has to be aborted. For instance, if the transaction was
speculatively executed and read value x=2, but later validation observes x=3,
the results of the transaction execution are no longer applicable and must
be discarded, while the transaction is marked for re-execution.

When an incarnation is aborted due to a validation failure, the entries in the
multi-version data-structure corresponding to its write-set are replaced with
a special ESTIMATE marker. This signifies that the next incarnation is estimated
to write to the same memory location, and is utilized for detecting potential
dependencies. In particular, an incarnation of transaction tx_j stops and waits
on a condition variable whenever it reads a value marked as an ESTIMATE that was
written by a lower transaction tx_k. When the execution of tx_k finishes, it
signals the condition variable and the execution of tx_j continues. This way,
tx_k does not read a value that is likely to cause an abort in the future due to a
validation failure, which would happen if the next incarnation of tx_k would
indeed write to the same location (the ESTIMATE markers that are not overwritten
are removed by the next incarnation).

The parallel executor relies on a collaborative scheduler in 'scheduler.rs',
which coordinates the validation and execution tasks among threads. Since the
preset serialization order dictates that the transactions must be committed in
order, a successful validation of an incarnation does not guarantee that it can
be committed. This is because an abort and re-execution of an earlier transaction
in the block might invalidate the incarnation read-set and necessitate
re-execution. Thus, when a transaction aborts, all higher transactions are
scheduled for re-validation. The same incarnation may be validated multiple times
and by different threads, potentially in parallel, but parallel execution ensures
that only the first abort per version is successful (the rest are ignored).
Since transactions must be committed in order, the scheduler prioritizes tasks
(validation and execution) associated with lower-indexed transactions.
Abstractly, the collaborative scheduler tracks an ordered set (priority queue w.o.
duplicates) V of pending validation tasks and an ordered set E of pending
execution tasks. Initially, V is empty and E contains execution tasks for
the initial incarnation of all transactions in the block. A transaction tx not
in E is either currently being executed or (its last incarnation) has completed.

Each thread repeats the following (loop in 'executor.rs'):
- Check done: if V and E are empty and no other thread is performing a task,
then return.
- Find next task: Perform the task with the smallest transaction index tx in V
and E:
  1. Execution task: Execute the next incarnation of tx. If a value marked as
     ESTIMATE is read, abort execution and add tx back to E. Otherwise:
     (a) If there is a write to a memory location to which the previous finished
         incarnation of tx has not written, create validation tasks for all
         transactions >= tx that are not currently in E or being executed and
         add them to V.
     (b) Otherwise, create a validation task only for tx and add it to V.
  2. Validation task: Validate the last incarnation of tx. If validation
     succeeds, continue. Otherwise, abort:
     (a) Mark every value (in the multi-versioned data-structure) written by
         the incarnation (that failed validation) as an ESTIMATE.
     (b) Create validation tasks for all transactions > tx that are not
         currently in E or being executed and add them to V.
     (c) Create an execution task for transaction tx with an incremented
         incarnation number, and add it to E.
When a transaction tx_k reads an ESTIMATE marker written by tx_j (with j < k),
we say that tx_k encounters a dependency (we treat tx_k as tx_j's dependency
because its read depends on a value that tx_j is estimated to write).
In the above description a transaction is added back to E immediately upon
encountering a dependency. However, we implement a slightly more involved
mechanism. Transaction tx_k is first recorded separately as a dependency of
tx_j, and only added back to E when the next incarnation of tx_j completes
(i.e. when the dependency is resolved).

In 'scheduler.rs', the ordered sets, V and E, are each implemented via a
single atomic counter coupled with a mechanism to track the status of
transactions, i.e. whether a given transaction is ready for validation or
execution, respectively. To pick a task, threads increment the smaller of these
counters until they find a task that is ready to be performed. To add a
(validation or execution) task for transaction tx, the thread updates the
status and reduces the corresponding counter to tx (if it had a larger value).
As an optimization in cases 1(b) and 2(c), instead of reducing the counter
value, the new task is returned back to the caller.

An incarnation of transaction might write to a memory location that was
previously read by an incarnation of a higher transaction according to the preset
serialization order. This is why in 1(a), when an incarnation finishes, new
validation tasks are created for higher transactions. Importantly, validation
tasks are scheduled optimistically, e.g. it is possible to concurrently validate
the latest incarnations of transactions tx_j, tx_{j+1}, tx_{j+2} and tx_{j+4}.
Suppose transactions tx_j, tx_{j+1} and tx_{j+4} are successfully validated,
while the validation of tx_{j+2} fails. When threads are available, parallel
execution capitalizes by performing these validations in parallel, allowing it
to detect the validation failure of tx_{j+2} faster in the above example
(at the expense of a validation of tx_{j+4} that needs to be redone).
Identifying validation failures and aborting incarnations as soon as possible
is crucial for the system performance, as any incarnation that reads values
written by a incarnation that aborts also needs to be aborted, forming a
cascade of aborts.

When an incarnation writes only to a subset of memory locations written by
the previously completed incarnation of the same transaction, i.e. case 1(b),
parallel execution schedules validation just for the incarnation itself.
This is sufficient because of 2(a), as the whole write-set of the previous
incarnation is marked as estimates during the abort. The abort then leads to
optimistically creating validation tasks for higher transactions in 2(b),
and threads that perform these tasks can already detect validation failures
due to the ESTIMATE markers on memory locations, instead of waiting for a
subsequent incarnation to finish.
**/
pub mod errors;
pub mod executor;
pub mod output_delta_resolver;
#[cfg(any(test, feature = "fuzzing"))]
pub mod proptest_types;
mod scheduler;
pub mod task;
mod txn_last_input_output;
#[cfg(test)]
mod unit_tests;

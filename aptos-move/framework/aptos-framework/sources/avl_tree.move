// copied from https://github.com/econia-labs/econia/blob/main/src/move/econia/sources/avl_queue.move

/// AVL queue: a hybrid between an AVL tree and a queue.
///
/// The present implementation involves an Adelson-Velsky and Landis
/// (AVL) tree, where each tree node has an enclosed doubly linked list.
/// Tree nodes correspond to keys from key-value insertion pairs, and
/// list nodes correspond to distinct insertion values sharing the same
/// insertion key. Hence tree node insertion keys are sorted in
/// lexicographical order, while list node insertion values are sorted
/// in order of insertion within a corresponding doubly linked list, to
/// the effect that key-value insertion pairs can be popped from the
/// head of the AVL queue in:
///
/// 1. Either ascending or descending order of insertion key (with
///    sort order set upon initialization), then by
/// 2. Ascending order of insertion within a doubly linked list.
///
/// Like an AVL tree, the present implementation also allows for
/// insertions and removals from anywhere inside the data structure.
///
/// # General overview sections
///
/// [AVL trees](#avl-trees)
///
/// * [Height invariant](#height-invariant)
/// * [Rotations](#rotations)
/// * [Retracing](#retracing)
/// * [As a map](#as-a-map)
///
/// [AVL queues](#avl-queues)
///
/// * [Key storage multiplicity](#key-storage-multiplicity)
/// * [Sort order](#sort-order)
/// * [Node structure](#node-structure)
/// * [Node provisioning](#node-provisioning)
/// * [Access keys](#access-keys)
/// * [Height](#height)
///
/// [Implementation analysis](#implementation-analysis)
///
/// * [Gas considerations](#gas-considerations)
/// * [Test development](#test-development)
/// * [Public function index](#public-function-index)
/// * [Dependency charts](#dependency-charts)
///
/// [Bit conventions](#bit-conventions)
///
/// * [Number](#number)
/// * [Status](#status)
/// * [Masking](#masking)
///
/// [References](#references)
///
/// [Complete DocGen index](#complete-docgen-index)
///
/// # AVL trees
///
/// ## Height invariant
///
/// An AVL tree is a self-balancing binary search tree where the height
/// of a node's two child subtrees differ by at most one. For example:
///
/// >         3
/// >        / \
/// >       2   5
/// >      /   / \
/// >     1   4   7
/// >            / \
/// >           6   8
///
/// Here, node 3's left child subtree has height 1 while its right child
/// subtree has height 2. Similarly, all other nodes satisfy the AVL
/// height invariant.
///
/// ## Rotations
///
/// Continuing the above example, if node 4 were to be removed then node
/// 5 would violate the height invariant:
///
/// >         3
/// >        / \
/// >       2   5
/// >      /     \
/// >     1       7
/// >            / \
/// >           6   8
///
/// Here, a left rotation is necessary to rebalance the tree, yielding:
///
/// >         3
/// >        / \
/// >       2   7
/// >      /   / \
/// >     1   5   8
/// >          \
/// >           6
///
/// Rotations are required whenever an insertion or removal leads to a
/// violation of the AVL height invariant.
///
/// ## Retracing
///
/// Similarly, insertion and removal operations may require retracing up
/// to the root, a process that updates node-wise state pertaining to
/// the height invariant.
///
/// Here, some implementations encode in each node a  "balance factor"
/// that describes whether a given node is left-heavy, right-heavy, or
/// balanced, while others track the height at a given node, as in the
/// present implementation. For example, consider the following tree:
///
/// >       2
/// >      / \
/// >     1   3
///
/// Here, nodes 1 and 3 have height 0, while node 2 has height 1.
/// Inserting 4 yields:
///
/// >       2
/// >      / \
/// >     1   3
/// >          \
/// >           4
///
/// Now node 4 has height 0, and the heights of nodes 3 and 2 have both
/// increased by 1. In practice, this means that inserting node 4 will
/// require modifying state in node 3 and in node 2, by looping back up
/// to the root to update heights, checking along the way whether a
/// height modification leads to an invariant violation.
///
/// ## As a map
///
/// AVL trees can be used as an associative array that maps from keys to
/// values, simply by storing values in the leaves of the tree. For
/// example, the insertion sequence
///
/// 1. $\langle 2, a \rangle$
/// 2. $\langle 3, b \rangle$
/// 3. $\langle 1, c \rangle$
/// 4. $\langle 4, d \rangle$
///
/// produces:
///
/// >          <2, a>
/// >          /    \
/// >     <1, c>    <3, b>
/// >                    \
/// >                    <4, d>
///
/// Notably, in an AVL tree, keys can only be inserted once, such that
/// inserting $\langle 2, e \rangle$ to the above tree would be invalid
/// unless $\langle 2, a \rangle$ were first removed.
///
/// # AVL queues
///
/// ## Key storage multiplicity
///
/// Unlike an AVL tree, which can only store one instance of a given
/// key, AVL queues can store multiple instances. For example, the
/// following insertion sequence, without intermediate removals, is
/// invalid in an AVL tree but valid in an AVL queue:
///
/// 1. $p_{3, 0} = \langle 3, 5 \rangle$
/// 2. $p_{3, 1} = \langle 3, 8 \rangle$
/// 3. $p_{3, 2} = \langle 3, 2 \rangle$
/// 4. $p_{3, 3} = \langle 3, 5 \rangle$
///
/// Here, the "key-value insertion pair"
/// $p_{i, j} = \langle i, v_j \rangle$ has:
///
/// * "Insertion key" $i$: the inserted key.
/// * "Insertion count" $j$: the number of key-value insertion pairs,
///   having the same insertion key, that were previously inserted.
/// * "Insertion value" $v_j$: the value from the key-value
///   insertion pair having insertion count $j$.
///
/// ## Sort order
///
/// Key-value insertion pairs in an AVL queue are sorted by:
///
/// 1. Either ascending or descending order of insertion key, then by
/// 2. Ascending order of insertion count.
///
/// For example, consider the key-value pair insertion pairs inserted in
/// the following sequence:
///
/// 1. $p_{1, 0} = \langle 1, a \rangle$
/// 2. $p_{3, 0} = \langle 3, b \rangle$
/// 3. $p_{3, 1} = \langle 3, c \rangle$
/// 4. $p_{1, 1} = \langle 1, d \rangle$
/// 5. $p_{2, 0} = \langle 2, e \rangle$
///
/// In an ascending AVL queue, the dequeue sequence would be:
///
/// 1. $p_{1, 0} = \langle 1, a \rangle$
/// 2. $p_{1, 1} = \langle 1, d \rangle$
/// 3. $p_{2, 0} = \langle 2, e \rangle$
/// 4. $p_{3, 0} = \langle 3, b \rangle$
/// 5. $p_{3, 1} = \langle 3, c \rangle$
///
/// In a descending AVL queue, the dequeue sequence would instead be:
///
/// 1. $p_{3, 0} = \langle 3, b \rangle$
/// 2. $p_{3, 1} = \langle 3, c \rangle$
/// 3. $p_{2, 0} = \langle 2, e \rangle$
/// 4. $p_{1, 0} = \langle 1, a \rangle$
/// 5. $p_{1, 1} = \langle 1, d \rangle$
///
/// ## Node structure
///
/// Continuing the above example, key-value insertion pairs would be
/// stored in an ascending AVL queue as follows:
///
/// >                              2 [e]
/// >                             / \
/// >                   [a -> d] 1   3 [b -> c]
/// >     AVL queue head ^                   ^ AVL queue tail
///
/// In a descending AVL queue:
///
/// >                         2 [e]
/// >                        / \
/// >              [a -> d] 1   3 [b -> c]
/// >     AVL queue tail ^         ^ AVL queue head
///
/// For each case, the tree node with insertion key 1 has a doubly
/// linked list where the head list node has insertion value a, and the
/// tail list node has insertion value d. Similarly, the tree node with
/// insertion key 3 has a doubly linked list where the head list node
/// has insertion value b, and the tail list node has insertion value c.
///
/// ## Node provisioning
///
/// Tree nodes and list nodes are stored as hash table entries, and thus
/// incur per-item global storage costs on the Aptos blockchain. As of
/// the time of this writing, per-item creation costs are by far the
/// most expensive per-item operation, and moreover, there is no
/// incentive to deallocate from memory. Hence the typical approach of
/// allocating a node upon insertion and deallocating upon removal is
/// more costly than the approach taken in the present implementation,
/// which involves re-using nodes once they have been allocated.
///
/// More specifically, when a tree node or list node is removed from the
/// AVL queue, it is pushed onto a stack of inactive nodes for the
/// corresponding type. Then, when an insertion operation requires a new
/// node, the inactive node can be popped off the top of the stack and
/// overwritten. Rather than allocating a new node for each insertion
/// and deallocating for each removal, this approach minimizes per-item
/// creation costs. Additionally, nodes can be pre-allocated upon AVL
/// queue initialization and pushed directly on the inactive nodes stack
/// so as to reduce per-item costs for future operations.
///
/// Since each active tree node contains a doubly linked list having at
/// least one active list node, the number of active tree nodes is
/// less than or equal to the number of active list nodes.
///
/// Tree nodes and list nodes are each assigned a 1-indexed 14-bit
/// serial ID known as a node ID. Node ID 0 is reserved for null, such
/// that the maximum number of allocated nodes for each node type is
/// thus $2^{14} - 1 = 16383$.
///
/// To additionally reduce costs, insertion values are not stored in
/// list nodes, but are also stored as hash table entries, accessed via
/// the corresponding list node ID. This approach reduces per-byte costs
/// associated with list node operations: if a list node is removed from
/// the middle of a doubly linked list, for example, per-byte write
/// costs will only be assessed on the next and last fields of the
/// removed node's neighbors, and will not be assessed on the neighbors'
/// insertion values.
///
/// ## Access keys
///
/// When a key-value insertion pair is inserted to the AVL queue, an
/// "access key" is returned, and can be used for subsequent lookup
/// operations. Access keys have the following bit structure:
///
/// | Bit(s) | Data                                         |
/// |--------|----------------------------------------------|
/// | 47-60  | Tree node ID                                 |
/// | 33-46  | List node ID                                 |
/// | 32     | If set, ascending AVL queue, else descending |
/// | 0-31   | Insertion key                                |
///
/// Insertion values are indexed by list node ID, and since the list
/// node ID for an insertion value is encoded in the access key
/// returned upon insertion, access keys can be used for $O(1)$ list
/// node lookup.
///
/// With the exception of list nodes at the head or tail of their
/// corresponding doubly linked list, list nodes do not, however,
/// indicate the corresponding tree node in which their doubly linked
/// list is located. This means that the corresponding tree node ID
/// and insertion key encoded in an access key are not verified from the
/// provided access key during lookup, as this process would require
/// $O(\log_2 n)$ lookup on the tree node.
///
/// Lookup operations thus assume that the provided access key
/// corresponds to a valid list node in the given AVL queue, and are
/// subject to undefined behavior if this condition is not met.
///
/// Notably, access keys are guaranteed to be unique within an AVL queue
/// at any given time, but are not guaranteed to be unique within an
/// AVL queue across time: since node IDs are reused per the stack-based
/// allocation strategy above, the same access key can be issued
/// multiple times. Hence it is up to callers to ensure appropriate
/// management of access keys, which effectively function as pointers
/// into AVL queue memory. Notably, if a caller wishes to uniquely
/// identify issued access keys, the caller can simply concatenate
/// access keys with a global counter.
///
/// Bits 0-32 are not required for lookup operations, but rather, are
/// included in access keys simply to provide additional metadata.
///
/// ## Height
///
/// In the present implementation, left or right height denotes the
/// height of a node's left or right subtree, respectively, plus one.
/// Subtree height is adjusted by one to avoid negative numbers, with
/// the resultant value denoting the height of a tree rooted at the
/// given node, accounting only for height to the given side. The height
/// of a node is denoted as the larger of its left height and right
/// height:
///
/// >       2
/// >      / \
/// >     1   3
/// >          \
/// >           4
///
/// | Key | Left height | Right height | Height |
/// |-----|-------------|--------------|--------|
/// | 1   | 0           | 0            | 0      |
/// | 2   | 1           | 2            | 2      |
/// | 3   | 0           | 1            | 1      |
/// | 4   | 0           | 0            | 0      |
///
/// The overall height $h$ of a tree (the height of the root node) is
/// related to the number of levels $l$ in the tree by the equation
/// $h = l - 1$, and for an AVL tree of size $n \geq 1$ nodes, the
/// number of levels in the tree lies in the interval
///
/// $$\log_2(n + 1) \leq l \leq c \log_2(n + d) + b$$
///
/// where
///
/// * $\varphi = \frac{1 + \sqrt{5}}{2} \approx 1.618$ (the golden
///   ratio),
/// * $c = \frac{1}{\log_2 \varphi} \approx 1.440$ ,
/// * $b = \frac{c}{2} \log_2 5 - 2 \approx -0.3277$ , and
/// * $d = 1 + \frac{1}{\varphi^4 \sqrt{5}} \approx 1.065$ .
///
/// With a maximum node count of $n_{max} = 2^{14} - 1 = 16383$, the
/// the maximum height $h_{max}$ of an AVL tree in the present
/// implementation is thus
///
/// $$h_{max} = \lfloor c \log_2(n_{max} + d) + b \rfloor - 1 = 18$$
///
/// such that left height and right height can always be encoded in
/// $b_{max} = \lceil \log_2 h_{max} \rceil = 5$ bits each.
///
/// Similarly, for a given height the size of an AVL tree is at most
///
/// $$log_2(n + 1) \leq h + 1$$
///
/// $$n + 1 \leq 2^{h + 1}$$
///
/// $$n \leq 2^{h + 1} - 1$$
///
/// and at least
///
/// $$c \log_2(n + d) + b \geq h + 1$$
///
/// $$\log_{\varphi}(n + d) + b \geq h + 1$$
///
/// $$\log_{\varphi}(n + d) \geq h + 1 - b$$
///
/// $$n + d \geq \varphi^{h + 1 - b}$$
///
/// $$n \geq \varphi^{h + 1 - b} - d$$
///
/// such that size lies in the interval
///
/// $$\varphi ^ {h +1 - b} - d \leq n \leq 2^{h + 1} - 1$$
///
/// which, for the special case of $h = 1$, results in the integer lower
/// bound
///
/// $$n_{h = 1} \geq \varphi ^ {1 + 1 - b} - d$$
///
/// $$n_{h = 1} \geq \varphi ^ {2 - b} - d$$
///
/// $$n_{h = 1} \geq \varphi ^ {2 - (\frac{c}{2}\log_2 5 - 2)} - d$$
///
/// $$n_{h = 1} \geq \varphi ^ {4 - \frac{c}{2}\log_2 5} - d$$
///
/// $$n_{h = 1} \geq \varphi ^ {4 - \frac{1}{2}\log_\varphi 5} - d$$
///
/// $$n_{h = 1} \geq \varphi ^ {4 - \log_\varphi \sqrt{5}} - d$$
///
/// $$n_{h = 1} \geq \varphi^4 / \varphi^{\log_\varphi \sqrt{5}} - d$$
///
/// $$n_{h = 1} \geq \varphi^4 / \sqrt{5} - d$$
///
/// $$n_{h = 1} \geq \varphi^4/\sqrt{5}-(1+1/(\varphi^4 \sqrt{5}))$$
///
/// $$n_{h=1}\geq(1+s)^4/(2^4s)-1-2^4/((1+s)^4s), s = \sqrt{5}$$
///
/// $$n_{h=1}\geq\frac{(1+s)^4}{2^4s}-\frac{2^4}{s(1+s)^4}-1$$
///
/// $$n_{h = 1} \geq 2$$
///
/// with the final step verifiable via a computer algebra system like
/// WolframAlpha. Thus for the heights possible in the present
/// implementation (and for one height higher):
///
/// | Height       | Minimum size | Maximum size    |
/// |--------------|--------------|-----------------|
/// | 0            | 1            | 1               |
/// | 1            | 2            | 3               |
/// | 2            | 4            | 7               |
/// | 3            | 7            | 15              |
/// | 4            | 12           | 31              |
/// | 5            | 20           | 63              |
/// | 6            | 33           | 127             |
/// | 7            | 54           | 255             |
/// | 8            | 88           | 511             |
/// | 9            | 143          | 1023            |
/// | 10           | 232          | 2047            |
/// | 11           | 376          | 4095            |
/// | 12           | 609          | 8191            |
/// | 13           | 986          | 16383 (`n_max`) |
/// | 14           | 1596         | 32767           |
/// | 15           | 2583         | 65535           |
/// | 16           | 4180         | 131071          |
/// | 17           | 6764         | 262143          |
/// | 18 (`h_max`) | 10945        | 524287          |
/// | 19           | 17710        | 1048575         |
///
/// Supporting Python calculations:
///
/// ```python
/// >>> import math
/// >>> phi = (1 + math.sqrt(5)) / 2
/// >>> phi
/// 1.618033988749895
/// >>> c = 1 / math.log(phi, 2)
/// >>> c
/// 1.4404200904125564
/// >>> b = c / 2 * math.log(5, 2) - 2
/// >>> b
/// -0.32772406181544556
/// >>> d = 1 + 1 / (phi ** 4 * math.sqrt(5))
/// >>> d
/// 1.0652475842498528
/// >>> n_max = 2 ** 14 - 1
/// >>> n_max
/// 16383
/// >>> h_max = math.floor(c * math.log(n_max + d, 2) + b) - 1
/// >>> h_max
/// 18
/// >>> b_max = math.ceil(math.log(h_max, 2))
/// >>> b_max
/// 5
/// >>> for h in range(h_max + 2):
/// ...     if h == 1:
/// ...         n_min = 2
/// ...     else:
/// ...         n_min = phi ** (h + 1 - b) - d
/// ...     n_max = 2 ** (h + 1) - 1
/// ...     n_min_ceil = math.ceil(n_min)
/// ...     print(f"h: {h}, n_min: {n_min_ceil}, n_max: {n_max}, "
/// ...           f"n_min_raw: {n_min}")
/// ...
/// h: 0, n_min: 1, n_max: 1, n_min_raw: 0.8291796067500634
/// h: 1, n_min: 2, n_max: 3, n_min_raw: 2
/// h: 2, n_min: 4, n_max: 7, n_min_raw: 3.894427190999917
/// h: 3, n_min: 7, n_max: 15, n_min_raw: 6.959674775249768
/// h: 4, n_min: 12, n_max: 31, n_min_raw: 11.919349550499538
/// h: 5, n_min: 20, n_max: 63, n_min_raw: 19.944271909999163
/// h: 6, n_min: 33, n_max: 127, n_min_raw: 32.92886904474855
/// h: 7, n_min: 54, n_max: 255, n_min_raw: 53.93838853899757
/// h: 8, n_min: 88, n_max: 511, n_min_raw: 87.93250516799598
/// h: 9, n_min: 143, n_max: 1023, n_min_raw: 142.93614129124342
/// h: 10, n_min: 232, n_max: 2047, n_min_raw: 231.93389404348926
/// h: 11, n_min: 376, n_max: 4095, n_min_raw: 375.9352829189826
/// h: 12, n_min: 609, n_max: 8191, n_min_raw: 608.9344245467217
/// h: 13, n_min: 986, n_max: 16383, n_min_raw: 985.9349550499542
/// h: 14, n_min: 1596, n_max: 32767, n_min_raw: 1595.9346271809259
/// h: 15, n_min: 2583, n_max: 65535, n_min_raw: 2582.93482981513
/// h: 16, n_min: 4180, n_max: 131071, n_min_raw: 4179.934704580306
/// h: 17, n_min: 6764, n_max: 262143, n_min_raw: 6763.934781979686
/// h: 18, n_min: 10945, n_max: 524287, n_min_raw: 10944.93473414424
/// h: 19, n_min: 17710, n_max: 1048575, n_min_raw: 17709.934763708177
/// ```
///
/// # Implementation analysis
///
/// ## Gas considerations
///
/// The present implementation relies on bit packing in assorted forms
/// to minimize per-byte storage costs: insertion keys are at most 32
/// bits and node IDs are 14 bits, for example, for maximum data
/// compression. Notably, associated bit packing operations are manually
/// inlined to reduce the number of function calls: as of the time of
/// this writing, instruction gas for 15 function calls costs the same
/// as a single per-item read out of global storage. Hence inlined
/// bit packing significantly reduces the number of function calls when
/// compared against an implementation with frequent calls to helper
/// functions of the form `mask_in_bits(target, incoming, shift)`.
///
/// As of the time of this writing, per-item reads and per-item writes
/// cost the same amount of storage gas, per-item writes cost 60 times
/// as much as per-byte writes, and per-byte writes cost approximately
/// 16.7 times as much as per-item writes. Hence with tree nodes only
/// occupying 128 bits (16 bytes), writing to a tree node only costs
/// about 25% more then reading a tree node.
///
/// With storage gas only assessed on a per-transaction basis, this
/// means that inserting a tree node and retracing all the way back up
/// to the root only costs 25% more than does the $O(\log_2 n)$ lookup
/// required for a key search, assuming no rebalancing takes place:
/// per-item read costs are assessed on the way down, then replaced by
/// per-item write costs on the way back up.
///
/// As for rebalancing, this process is only (potentially) required for
/// operations that alter the number of tree nodes: if key-value
/// insertion pair operations consistently involve the same insertion
/// keys, then tree retracing and rebalancing operations are minimized.
///
/// In the case that rebalancing does occur, per-item write costs on the
/// affected nodes are essentially amortized against the gas reductions
/// afforded by an AVL tree's height guarantees: the height of a
/// red-black tree is at most $2 \log_2 n$, for example, but the
/// height of an AVL tree is at most approximately $1.44 \log_2 n$
/// (per above). This means that fewer per-item costs are assessed on
/// key lookup for the latter, and moreover, re-coloring operations
/// required for the former may still require looping back up to the
/// root in the worst case, resulting in higher per-item write costs.
///
/// ## Test development
///
/// Unit tests for the present implementation were written alongside
/// source code, with some integration refactors applied along the way.
/// For example, rotation tests were first devised based on manual
/// allocation of nodes, then some were later updated for specific
/// insertion and deletion scenarios. As such, syntax may vary
/// slightly between some test cases depending on the level to which
/// they were later scoped for integration.
///
/// ## Public function index
///
/// * `borrow()`
/// * `borrow_head()`
/// * `borrow_head_mut()`
/// * `borrow_mut()`
/// * `borrow_tail()`
/// * `borrow_tail_mut()`
/// * `contains_active_list_node_id()`
/// * `get_access_key_insertion_key()`
/// * `get_head_key()`
/// * `get_height()`
/// * `get_tail_key()`
/// * `has_key()`
/// * `insert()`
/// * `insert_check_eviction()`
/// * `insert_evict_tail()`
/// * `is_ascending()`
/// * `is_ascending_access_key()`
/// * `is_empty()`
/// * `is_local_tail()`
/// * `new()`
/// * `next_list_node_id_in_access_key()`
/// * `pop_head()`
/// * `pop_tail()`
/// * `remove()`
/// * `would_update_head()`
/// * `would_update_tail()`
///
/// ## Dependency charts
///
/// The below dependency charts use `mermaid.js` syntax, which can be
/// automatically rendered into a diagram (depending on the browser)
/// when viewing the documentation file generated from source code. If
/// a browser renders the diagrams with coloring that makes it difficult
/// to read, try a different browser.
///
/// `retrace()`:
///
/// ```mermaid
///
/// flowchart LR
///
/// retrace --> retrace_update_heights
/// retrace --> retrace_rebalance
/// retrace --> retrace_prep_iterate
///
/// retrace_rebalance --> retrace_rebalance_rotate_left_right
/// retrace_rebalance --> retrace_rebalance_rotate_right
/// retrace_rebalance --> retrace_rebalance_rotate_right_left
/// retrace_rebalance --> retrace_rebalance_rotate_left
///
/// ```
///
/// `insert()`:
///
/// ```mermaid
///
/// flowchart LR
///
/// insert --> search
/// insert --> insert_list_node
/// insert --> insert_tree_node
/// insert --> retrace
/// insert --> insert_check_head_tail
///
/// insert_list_node --> insert_list_node_get_last_next
/// insert_list_node --> insert_list_node_assign_fields
///
/// insert_tree_node --> insert_tree_node_update_parent_edge
///
/// ```
///
/// `remove()`:
///
/// ```mermaid
///
/// flowchart LR
///
/// remove --> remove_list_node
/// remove --> remove_update_head
/// remove --> remove_update_tail
/// remove --> remove_tree_node
///
/// remove_list_node --> remove_list_node_update_edges
///
/// remove_update_head --> traverse
///
/// remove_update_tail --> traverse
///
/// remove_tree_node --> remove_tree_node_with_children
/// remove_tree_node --> remove_tree_node_follow_up
///
/// remove_tree_node_follow_up --> retrace
///
/// ```
///
/// Assorted:
///
/// ```mermaid
///
/// flowchart LR
///
/// insert_evict_tail --> insert
/// insert_evict_tail --> remove
///
/// insert_check_eviction --> remove
/// insert_check_eviction --> insert
///
/// next_list_node_id_in_access_key --> traverse
///
/// has_key --> search
///
/// pop_head --> remove
///
/// pop_tail --> remove
///
/// ```
///
/// # Bit conventions
///
/// ## Number
///
/// Bit numbers are 0-indexed from the least-significant bit (LSB):
///
/// >     11101...1010010101
/// >       bit 5 = 0 ^    ^ bit 0 = 1
///
/// ## Status
///
/// `0` is considered an "unset" bit, and `1` is considered a "set" bit.
/// Hence `11101` is set at bit 0 and unset at bit 1.
///
/// ## Masking
///
/// In the present implementation, a bitmask refers to a bitstring that
/// is only set at the indicated bit. For example, a bitmask with bit 0
/// set corresponds to `000...001`, and a bitmask with bit 3 set
/// corresponds to `000...01000`.
///
/// # References
///
/// * [Adelson-Velsky and Landis 1962] (original paper)
/// * [Galles 2011] (interactive visualizer)
/// * [Wikipedia 2022]
///
/// [Adelson-Velsky and Landis 1962]:
///     https://zhjwpku.com/assets/pdf/AED2-10-avl-paper.pdf
/// [Galles 2011]:
///     https://www.cs.usfca.edu/~galles/visualization/AVLtree.html
/// [Wikipedia 2022]:
///     https://en.wikipedia.org/wiki/AVL_tree
///
/// # Complete DocGen index
///
/// The below index is automatically generated from source code:
module aptos_framework::avl_queue {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_std::table::{Self, Table};
    use aptos_std::table_with_length::{Self, TableWithLength};
    use std::option::{Self, Option};

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use std::vector;

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// A hybrid between an AVL tree and a queue. See above.
    ///
    /// Most non-table fields stored compactly in `bits` as follows:
    ///
    /// | Bit(s)  | Data                                               |
    /// |---------|----------------------------------------------------|
    /// | 126     | If set, ascending AVL queue, else descending       |
    /// | 112-125 | Tree node ID at top of inactive stack              |
    /// | 98-111  | List node ID at top of inactive stack              |
    /// | 84-97   | AVL queue head list node ID                        |
    /// | 52-83   | AVL queue head insertion key (if node ID not null) |
    /// | 38-51   | AVL queue tail list node ID                        |
    /// | 6-37    | AVL queue tail insertion key (if node ID not null) |
    /// | 0-5     | Bits 8-13 of tree root node ID                     |
    ///
    /// Bits 0-7 of the tree root node ID are stored in `root_lsbs`.
    struct AVLqueue<V> has store {
        bits: u128,
        root_lsbs: u8,
        /// Map from tree node ID to tree node.
        tree_nodes: TableWithLength<u64, TreeNode>,
        /// Map from list node ID to list node.
        list_nodes: TableWithLength<u64, ListNode>,
        /// Map from list node ID to optional insertion value.
        values: Table<u64, Option<V>>
    }

    /// A tree node in an AVL queue.
    ///
    /// All fields stored compactly in `bits` as follows:
    ///
    /// | Bit(s) | Data                                 |
    /// |--------|--------------------------------------|
    /// | 94-125 | Insertion key                        |
    /// | 89-93  | Left height                          |
    /// | 84-88  | Right height                         |
    /// | 70-83  | Parent node ID                       |
    /// | 56-69  | Left child node ID                   |
    /// | 42-55  | Right child node ID                  |
    /// | 28-41  | List head node ID                    |
    /// | 14-27  | List tail node ID                    |
    /// | 0-13   | Next inactive node ID, when in stack |
    ///
    /// All fields except next inactive node ID are ignored when the
    /// node is in the inactive nodes stack.
    struct TreeNode has store {
        bits: u128
    }

    /// A list node in an AVL queue.
    ///
    /// For compact storage, a "virtual last field" and a "virtual next
    /// field" are split into two `u8` fields each: one for
    /// most-significant bits (`last_msbs`, `next_msbs`), and one for
    /// least-significant bits (`last_lsbs`, `next_lsbs`).
    ///
    /// When set at bit 14, the 16-bit concatenated result of `_msbs`
    /// and `_lsbs` fields, in either case, refers to a tree node ID: If
    /// `last_msbs` and `last_lsbs` indicate a tree node ID, then the
    /// list node is the head of the list at the given tree node. If
    /// `next_msbs` and `next_lsbs` indicate a tree node ID, then the
    /// list node is the tail of the list at the given tree node.
    ///
    /// If not set at bit 14, the corresponding node ID is either the
    /// last or the next list node in the doubly linked list.
    ///
    /// If list node is in the inactive list node stack, next node ID
    /// indicates next inactive node in the stack.
    struct ListNode has store {
        last_msbs: u8,
        last_lsbs: u8,
        next_msbs: u8,
        next_lsbs: u8
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Number of allocated tree nodes is too high.
    const E_TOO_MANY_TREE_NODES: u64 = 0;
    /// Number of allocated list nodes is too high.
    const E_TOO_MANY_LIST_NODES: u64 = 1;
    /// Insertion key is too large.
    const E_INSERTION_KEY_TOO_LARGE: u64 = 2;
    /// Attempted insertion with eviction from empty AVL queue.
    const E_EVICT_EMPTY: u64 = 3;
    /// Attempted insertion with eviction for key-value insertion pair
    /// that would become new tail.
    const E_EVICT_NEW_TAIL: u64 = 4;
    /// Specified height exceeds max height.
    const E_INVALID_HEIGHT: u64 = 5;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Ascending AVL queue flag.
    const ASCENDING: bool = true;
    /// Bit flag denoting ascending AVL queue.
    const BIT_FLAG_ASCENDING: u8 = 1;
    /// Bit flag denoting a tree node.
    const BIT_FLAG_TREE_NODE: u8 = 1;
    /// Number of bits in a byte.
    const BITS_PER_BYTE: u8 = 8;
    /// Flag for decrement to height during retrace.
    const DECREMENT: bool = false;
    /// Descending AVL queue flag.
    const DESCENDING: bool = false;
    /// `u64` bitmask with all bits set, generated in Python via
    /// `hex(int('1' * 64, 2))`.
    const HI_64: u64 = 0xffffffffffffffff;
    /// `u128` bitmask with all bits set, generated in Python via
    /// `hex(int('1' * 128, 2))`.
    const HI_128: u128 = 0xffffffffffffffffffffffffffffffff;
    /// Single bit set in integer of width required to encode bit flag.
    const HI_BIT: u8 = 1;
    /// All bits set in integer of width required to encode a byte.
    /// Generated in Python via `hex(int('1' * 8, 2))`.
    const HI_BYTE: u64 = 0xff;
    /// All bits set in integer of width required to encode left or
    /// right height. Generated in Python via `hex(int('1' * 5, 2))`.
    const HI_HEIGHT: u8 = 0x1f;
    /// All bits set in integer of width required to encode insertion
    /// key. Generated in Python via `hex(int('1' * 32, 2))`.
    const HI_INSERTION_KEY: u64 = 0xffffffff;
    /// All bits set in integer of width required to encode node ID.
    /// Generated in Python via `hex(int('1' * 14, 2))`.
    const HI_NODE_ID: u64 = 0x3fff;
    /// Flag for increment to height during retrace.
    const INCREMENT: bool = true;
    /// Flag for left direction.
    const LEFT: bool = true;
    /// Maximum tree height.
    const MAX_HEIGHT: u8 = 18;
    /// Flag for null value when null defined as 0.
    const NIL: u8 = 0;
    /// $2^{14} - 1$, the maximum number of nodes that can be allocated
    /// for either node type.
    const N_NODES_MAX: u64 = 16383;
    /// Flag for inorder predecessor traversal.
    const PREDECESSOR: bool = true;
    /// Flag for right direction.
    const RIGHT: bool = false;
    /// Number of bits sort order bit flag is shifted in an access key.
    const SHIFT_ACCESS_SORT_ORDER: u8 = 32;
    /// Number of bits list node ID is shifted in an access key.
    const SHIFT_ACCESS_LIST_NODE_ID: u8 = 33;
    /// Number of bits tree node ID is shifted in an access key.
    const SHIFT_ACCESS_TREE_NODE_ID: u8 = 47;
    /// Number of bits sort order is shifted in `AVLqueue.bits`.
    const SHIFT_SORT_ORDER: u8 = 126;
    /// Number of bits left child node ID is shifted in `TreeNode.bits`.
    const SHIFT_CHILD_LEFT: u8 = 56;
    /// Number of bits right child node ID is shifted in
    /// `TreeNode.bits`.
    const SHIFT_CHILD_RIGHT: u8 = 42;
    /// Number of bits AVL queue head insertion key is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_HEAD_KEY: u8 = 52;
    /// Number of bits AVL queue head list node ID is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_HEAD_NODE_ID: u8 = 84;
    /// Number of bits left height is shifted in `TreeNode.bits`.
    const SHIFT_HEIGHT_LEFT: u8 = 89;
    /// Number of bits right height is shifted in `TreeNode.bits`.
    const SHIFT_HEIGHT_RIGHT: u8 = 84;
    /// Number of bits insertion key is shifted in `TreeNode.bits`.
    const SHIFT_INSERTION_KEY: u8 = 94;
    /// Number of bits inactive list node stack top is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_LIST_STACK_TOP: u8 = 98;
    /// Number of bits node type bit flag is shifted in `ListNode`
    /// virtual last and next fields.
    const SHIFT_NODE_TYPE: u8 = 14;
    /// Number of bits list head node ID is shifted in `TreeNode.bits`.
    const SHIFT_LIST_HEAD: u8 = 28;
    /// Number of bits list tail node ID is shifted in `TreeNode.bits`.
    const SHIFT_LIST_TAIL: u8 = 14;
    /// Number of bits parent node ID is shifted in `AVLqueue.bits`.
    const SHIFT_PARENT: u8 = 70;
    /// Number of bits AVL queue tail insertion key is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_TAIL_KEY: u8 = 6;
    /// Number of bits AVL queue tail list node ID is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_TAIL_NODE_ID: u8 = 38;
    /// Number of bits inactive tree node stack top is shifted in
    /// `AVLqueue.bits`.
    const SHIFT_TREE_STACK_TOP: u8 = 112;
    /// Flag for inorder successor traversal.
    const SUCCESSOR: bool = false;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Immutably borrow insertion value corresponding to access key,
    /// aborting if invalid key.
    ///
    /// # Assumptions
    ///
    /// * Provided access key corresponds to a valid list node in the
    ///   given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow<V>(
        avlq_ref: &AVLqueue<V>,
        access_key: u64
    ): &V {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        // Immutably borrow corresponding insertion value.
        option::borrow(table::borrow(&avlq_ref.values, list_node_id))
    }

    /// Immutably borrow AVL queue head insertion value, aborting if
    /// empty.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow_head<V>(
        avlq_ref: &AVLqueue<V>
    ): &V {
        let (list_node_id) = (((avlq_ref.bits >> SHIFT_HEAD_NODE_ID) &
            (HI_NODE_ID as u128)) as u64); // Get head list node ID.
        // Immutably borrow corresponding insertion value.
        option::borrow(table::borrow(&avlq_ref.values, list_node_id))
    }

    /// Mutably borrow AVL queue head insertion value, aborting if
    /// empty.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow_head_mut<V>(
        avlq_ref_mut: &mut AVLqueue<V>
    ): &mut V {
        let (list_node_id) = (((avlq_ref_mut.bits >> SHIFT_HEAD_NODE_ID) &
            (HI_NODE_ID as u128)) as u64); // Get head list node ID.
        // Mutably borrow corresponding insertion value.
        option::borrow_mut(
            table::borrow_mut(&mut avlq_ref_mut.values, list_node_id))
    }

    /// Mutably borrow insertion value corresponding to access key,
    /// aborting if invalid key.
    ///
    /// # Assumptions
    ///
    /// * Provided access key corresponds to a valid list node in the
    ///   given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow_mut<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        access_key: u64
    ): &mut V {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        // Mutably borrow corresponding insertion value.
        option::borrow_mut(
            table::borrow_mut(&mut avlq_ref_mut.values, list_node_id))
    }

    /// Immutably borrow AVL queue tail insertion value, aborting if
    /// empty.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow_tail<V>(
        avlq_ref: &AVLqueue<V>
    ): &V {
        let (list_node_id) = (((avlq_ref.bits >> SHIFT_TAIL_NODE_ID) &
            (HI_NODE_ID as u128)) as u64); // Get tail list node ID.
        // Immutably borrow corresponding insertion value.
        option::borrow(table::borrow(&avlq_ref.values, list_node_id))
    }

    /// Mutably borrow AVL queue tail insertion value, aborting if
    /// empty.
    ///
    /// # Testing
    ///
    /// * `test_borrow_borrow_mut()`
    public fun borrow_tail_mut<V>(
        avlq_ref_mut: &mut AVLqueue<V>
    ): &mut V {
        let (list_node_id) = (((avlq_ref_mut.bits >> SHIFT_TAIL_NODE_ID) &
            (HI_NODE_ID as u128)) as u64); // Get tail list node ID.
        // Mutably borrow corresponding insertion value.
        option::borrow_mut(
            table::borrow_mut(&mut avlq_ref_mut.values, list_node_id))
    }

    /// Return `true` if list node ID encoded in `access_key` is active.
    ///
    /// # Testing
    ///
    /// * `test_contains_active_list_node_id()`
    public fun contains_active_list_node_id<V>(
        avlq_ref: &AVLqueue<V>,
        access_key: u64
    ): bool {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        // Return false if no list node in AVL queue with list node ID,
        if (!table::contains(&avlq_ref.values, list_node_id)) false else
        // Otherwise, return if there is an insertion value for
        // given list node ID.
            option::is_some(table::borrow(&avlq_ref.values, list_node_id))
    }

    /// Get insertion key encoded in an access key.
    ///
    /// # Testing
    ///
    /// * `test_access_key_getters()`
    public fun get_access_key_insertion_key(
        access_key: u64
    ): u64 {
        access_key & HI_INSERTION_KEY
    }

    /// Return none if AVL queue empty, else head insertion key.
    ///
    /// # Testing
    ///
    /// * `test_get_head_tail_key()`
    public fun get_head_key<V>(
        avlq_ref: &AVLqueue<V>
    ): Option<u64> {
        let bits = avlq_ref.bits; // Get AVL queue bits.
        // Get AVL queue head node ID and insertion key fields.
        let (avlq_head_node_id, avlq_head_insertion_key) =
            ((((bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_HEAD_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // If no AVL queue head return none, else head insertion key.
        if (avlq_head_node_id == (NIL as u64)) option::none() else
            option::some(avlq_head_insertion_key)
    }

    /// Return none if empty AVL queue, else tree height.
    ///
    /// # Reference diagram
    ///
    /// Height 0 for sole node at root.
    ///
    /// >     4
    ///
    /// Insert 5, increasing right height to 1:
    ///
    /// >     4
    /// >      \
    /// >       5
    ///
    /// Insert 3, increasing left height to 1 as well:
    ///
    /// >       4
    /// >      / \
    /// >     3   5
    ///
    /// Insert 1, increasing left height to 2:
    ///
    /// >         4
    /// >        / \
    /// >       3   5
    /// >      /
    /// >     1
    ///
    /// # Testing
    ///
    /// * `test_get_height()`
    public fun get_height<V>(
        avlq_ref: &AVLqueue<V>
    ): Option<u8> {
        // Get root MSBs.
        let msbs = avlq_ref.bits & ((HI_NODE_ID as u128) >> BITS_PER_BYTE);
        let root = ((msbs << BITS_PER_BYTE) as u64) |
            (avlq_ref.root_lsbs as u64); // Mask in root LSBs.
        // Return none if no root and thus empty AVL queue.
        if (root == (NIL as u64)) return option::none();
        // Immutably borrow root node.
        let root_ref = table_with_length::borrow(&avlq_ref.tree_nodes, root);
        let bits = root_ref.bits; // Get root bits.
        let (height_left, height_right) = // Get left and right height.
            ((((bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8));
        let height = // Height is greater of left and right height.
            if (height_left >= height_right) height_left else height_right;
        option::some(height) // Return option-packed height.
    }

    /// Return none if AVL queue empty, else tail insertion key.
    ///
    /// # Testing
    ///
    /// * `test_get_head_tail_key()`
    public fun get_tail_key<V>(
        avlq_ref: &AVLqueue<V>
    ): Option<u64> {
        let bits = avlq_ref.bits; // Get AVL queue bits.
        // Get AVL queue tail node ID and insertion key fields.
        let (avlq_tail_node_id, avlq_tail_insertion_key) =
            ((((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_TAIL_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // If no AVL queue tail return none, else tail insertion key.
        if (avlq_tail_node_id == (NIL as u64)) option::none() else
            option::some(avlq_tail_insertion_key)
    }

    /// Return `true` if insertion `key` in AVL queue, else `false`.
    ///
    /// # Aborts
    ///
    /// * `E_INSERTION_KEY_TOO_LARGE`: Insertion key is too large.
    ///
    /// # Testing
    ///
    /// * `test_has_key()`
    /// * `test_has_key_too_big()`
    public fun has_key<V>(
        avlq_ref: &AVLqueue<V>,
        key: u64
    ): bool {
        // Assert insertion key is not too many bits.
        assert!(key <= HI_INSERTION_KEY, E_INSERTION_KEY_TOO_LARGE);
        // Search for key, storing match flags.
        let (nil_if_empty, none_if_found_or_empty) = search(avlq_ref, key);
        // Return true if found, else false.
        if ((nil_if_empty != (NIL as u64)) &&
            option::is_none(&none_if_found_or_empty)) true else false
    }

    /// Insert a key-value pair into an AVL queue.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `key`: Key to insert.
    /// * `value`: Value to insert.
    ///
    /// # Returns
    ///
    /// * `u64`: Access key used for lookup.
    ///
    /// # Aborts
    ///
    /// * `E_INSERTION_KEY_TOO_LARGE`: Insertion key is too large.
    ///
    /// # Failure testing
    ///
    /// * `test_insert_insertion_key_too_large()`
    /// * `test_insert_too_many_list_nodes()`
    ///
    /// # State verification testing
    ///
    /// See `test_insert()` for state verification testing of the
    /// below insertion sequence.
    ///
    /// Insert $\langle 3, 9 \rangle$:
    ///
    /// >     3 [9]
    ///
    /// Insert $\langle 4, 8 \rangle$:
    ///
    /// >     3 [9]
    /// >      \
    /// >       4 [8]
    ///
    /// Insert $\langle 5, 7 \rangle$:
    ///
    /// >           4 [8]
    /// >          / \
    /// >     [9] 3   5 [7]
    ///
    /// Insert $\langle 3, 6 \rangle$
    ///
    /// >                4 [8]
    /// >               / \
    /// >     [9 -> 6] 3   5 [7]
    ///
    /// Insert $\langle 5, 5 \rangle$
    ///
    /// >                4 [8]
    /// >               / \
    /// >     [9 -> 6] 3   5 [7 -> 5]
    public fun insert<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64,
        value: V
    ): u64 {
        // Assert insertion key is not too many bits.
        assert!(key <= HI_INSERTION_KEY, E_INSERTION_KEY_TOO_LARGE);
        // Search for key, storing match node ID, and optional side on
        // which a new leaf would be inserted relative to match node.
        let (match_node_id, new_leaf_side) = search(avlq_ref_mut, key);
        // If search returned null from the root, or if search flagged
        // that a new tree node will have to be inserted as child, flag
        // that the inserted list node will be the sole node in the
        // corresponding doubly linked list.
        let solo = match_node_id == (NIL as u64) ||
            option::is_some(&new_leaf_side);
        // If a solo list node, flag no anchor tree node yet inserted,
        // otherwise set anchor tree node as match node from search.
        let anchor_tree_node_id = if (solo) (NIL as u64) else match_node_id;
        let list_node_id = // Insert list node, storing its node ID.
            insert_list_node(avlq_ref_mut, anchor_tree_node_id, value);
        // Get corresponding tree node: if solo list node, insert a tree
        // node and store its ID. Otherwise tree node is match node from
        // search.
        let tree_node_id = if (solo) insert_tree_node(
            avlq_ref_mut, key, match_node_id, list_node_id, new_leaf_side) else
            match_node_id;
        // If just inserted new tree node that is not root, retrace
        // starting at the parent to the inserted tree node.
        if (solo && (match_node_id != (NIL as u64)))
            retrace(avlq_ref_mut, match_node_id, INCREMENT,
                *option::borrow(&new_leaf_side));
        // Check AVL queue head and tail.
        insert_check_head_tail(avlq_ref_mut, key, list_node_id);
        let order_bit = // Get sort order bit from AVL queue bits.
            (avlq_ref_mut.bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128);
        // Return bit-packed access key.
        key | ((order_bit as u64) << SHIFT_ACCESS_SORT_ORDER) |
            ((list_node_id) << SHIFT_ACCESS_LIST_NODE_ID) |
            ((tree_node_id) << SHIFT_ACCESS_TREE_NODE_ID)
    }

    /// Try inserting key-value pair, evicting AVL queue tail as needed.
    ///
    /// If AVL queue is empty then no eviction is required, and a
    /// standard insertion is performed.
    ///
    /// If AVL queue is not empty, then eviction is required if the AVL
    /// queue is above the provided critical height or if the maximum
    /// number of list nodes have already been allocated and all are
    /// active. Here, insertion is not permitted if attempting to insert
    /// a new tail. Otherwise, the tail of the AVL queue is removed then
    /// the provided key-value pair is inserted.
    ///
    /// If AVL queue is not empty but eviction is not required, a
    /// standard insertion is performed.
    ///
    /// Does not guarantee that height will be less than or equal to
    /// critical height post-insertion, since there is no limit on the
    /// number of list nodes with a given insertion key: evicting the
    /// tail node does not guarantee removing a corresponding tree node.
    /// Rather, critical height is simply a threshold for determining
    /// whether height-driven eviction is required.
    ///
    /// Does not check number of active tree nodes because the number of
    /// active tree nodes is less than or equal to the number of active
    /// list nodes.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `key`: Key to insert.
    /// * `value`: Value to insert.
    /// * `critical_height`: Tree height above which eviction should
    ///   take place.
    ///
    /// # Returns
    ///
    /// * `u64`: Access key of key-value pair just inserted, otherwise
    ///   `NIL` if an invalid insertion.
    /// * `u64`: `NIL` if no eviction required, otherwise access key of
    ///   evicted key-value insertion pair.
    /// * `Option<V>`: None if no eviction required. If an invalid
    ///   insertion, the insertion value that could not be inserted.
    ///   Otherwise, the evicted insertion value.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_HEIGHT`: Specified height exceeds max height.
    ///
    /// # Reference diagrams
    ///
    /// ## Case 1
    ///
    /// * Ascending AVL queue.
    /// * Left height greater than or equal to right height.
    /// * Max list nodes active.
    ///
    /// >       [1] 2
    /// >          / \
    /// >     [2] 1   3 [3 -> 4 -> ... N_NODES_MAX]
    ///
    /// 1. Attempting to insert with insertion key 3, critical height 2
    ///    is invalid (not too tall, max list nodes active, attempting
    ///    to insert tail).
    /// 2. Attempting to insert with insertion key 2, critical height 2
    ///    then evicts tail (not too tall, max list nodes active, not
    ///    attempting to insert tail).
    ///
    /// ## Case 2
    ///
    /// * Descending AVL queue.
    /// * Left height not greater than or equal to right height.
    /// * Not max list nodes active.
    ///
    /// >       [123] 2
    /// >            / \
    /// >     [456] 1   3 [789]
    /// >                \
    /// >                 4 [321]
    ///
    /// 1. Attempting to insert with insertion key 1, critical height 1
    ///    is invalid (too tall, not max list nodes active, attempting
    ///    to insert tail).
    /// 2. Attempting to insert with insertion key 2, critical height 1
    ///    then evicts tail (too tall, not max list nodes active, not
    ///    attempting to insert tail).
    /// 3. Attempting to insert with insertion key 1, critical height
    ///    10 then results in standard insertion at tail (not too tall,
    ///    not max list nodes active).
    ///
    /// # Testing
    ///
    /// * `test_insert_check_eviction_case_1()`
    /// * `test_insert_check_eviction_case_2()`
    /// * `test_insert_check_eviction_empty()`
    /// * `test_insert_check_eviction_invalid_height()`
    public fun insert_check_eviction<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64,
        value: V,
        critical_height: u8
    ): (
        u64,
        u64,
        Option<V>
    ) {
        // Assert specified critical height is a valid height.
        assert!(critical_height <= MAX_HEIGHT, E_INVALID_HEIGHT);
        let bits = avlq_ref_mut.bits; // Get AVL queue bits.
        let tail_list_node_id = // Get AVL queue tail list node id.
            (((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64);
        // If empty, return result of standard insertion.
        if (tail_list_node_id == (NIL as u64)) return
            (insert(avlq_ref_mut, key, value), (NIL as u64), option::none());
        // Get inactive list nodes stack top and root MSBs.
        let (list_top, root_msbs) =
            ((((bits >> SHIFT_LIST_STACK_TOP) & (HI_NODE_ID as u128)) as u64),
                (((bits & ((HI_NODE_ID as u128) >> BITS_PER_BYTE)))));
        // Get root field by masking in root LSBs.
        let root = ((root_msbs << BITS_PER_BYTE) as u64) |
            (avlq_ref_mut.root_lsbs as u64);
        let root_ref = // Immutably borrow root node.
            table_with_length::borrow(&avlq_ref_mut.tree_nodes, root);
        let r_bits = root_ref.bits; // Get root node bits.
        let (height_left, height_right) = // Get left and right height.
            ((((r_bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((r_bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8));
        let height = // Height is greater of left and right height.
            if (height_left >= height_right) height_left else height_right;
        let too_tall = height > critical_height; // Check if too tall.
        // Get number of allocated list nodes.
        let n_list_nodes = table_with_length::length(&avlq_ref_mut.list_nodes);
        let max_list_nodes_active = // Check if max list nodes active.
            (n_list_nodes == N_NODES_MAX) && (list_top == (NIL as u64));
        // Declare tail access key and insertion value.
        let (tail_access_key, tail_value);
        // If above critical height or max list nodes active:
        if (too_tall || max_list_nodes_active) {
            // If need to evict:
            let order_bit = // Get sort order bit flag.
                (((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8);
            // Determine if ascending AVL queue.
            let ascending = order_bit == BIT_FLAG_ASCENDING;
            // Get AVL queue tail insertion key.
            let tail_key = (((bits >> SHIFT_TAIL_KEY) &
                (HI_INSERTION_KEY as u128)) as u64);
            // If ascending and insertion key greater than or equal to
            // tail key, or descending and insertion key less than or
            // equal to tail key, attempting to insert new tail: invalid
            // when above critical height or max list nodes active.
            if ((ascending && (key >= tail_key)) ||
                (!ascending && (key <= tail_key))) return
                ((NIL as u64), (NIL as u64), option::some(value));
            // Immutably borrow tail list node.
            let tail_list_node_ref = table_with_length::borrow(
                &avlq_ref_mut.list_nodes, tail_list_node_id);
            let next = // Get virtual next field from node.
                ((tail_list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
                    (tail_list_node_ref.next_lsbs as u64);
            // Get tree node ID encoded in next field.
            let tail_tree_node_id = next & (HI_NODE_ID as u64);
            tail_access_key = tail_key | // Get tail access key.
                ((order_bit as u64) << SHIFT_ACCESS_SORT_ORDER) |
                ((tail_list_node_id) << SHIFT_ACCESS_LIST_NODE_ID) |
                ((tail_tree_node_id) << SHIFT_ACCESS_TREE_NODE_ID);
            // Get tail insertion value from evicted tail.
            tail_value = option::some(remove(avlq_ref_mut, tail_access_key));
        } else {
            // If no potential for eviction:
            // Flag no evicted tail return values.
            (tail_access_key, tail_value) = ((NIL as u64), option::none());
        }; // Optional eviction now complete.
        // Return access key for new key-value insertion pair, optional
        // access key and insertion value for evicted tail.
        (insert(avlq_ref_mut, key, value), tail_access_key, tail_value)
    }

    /// Insert key-value insertion pair, evicting AVL queue tail.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `key`: Key to insert.
    /// * `value`: Value to insert.
    ///
    /// # Returns
    ///
    /// * `u64`: Access key for lookup of inserted pair.
    /// * `u64`: Access key of evicted tail.
    /// * `V`: Evicted tail insertion value.
    ///
    /// # Aborts
    ///
    /// * `E_EVICT_EMPTY`: AVL queue is empty.
    /// * `E_EVICT_NEW_TAIL`: Key-value insertion pair would itself
    ///   become new tail.
    ///
    /// # Testing
    ///
    /// * `test_insert_evict_tail()`
    /// * `test_insert_evict_tail_empty()`
    /// * `test_insert_evict_tail_new_tail_ascending()`
    /// * `test_insert_evict_tail_new_tail_descending()`
    public fun insert_evict_tail<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64,
        value: V
    ): (
        u64,
        u64,
        V
    ) {
        let bits = avlq_ref_mut.bits; // Get AVL queue bits.
        // Get AVL queue sort order bit, tail list node ID, and
        // tail insertion key.
        let (order_bit, tail_list_node_id, tail_key) =
            ((((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8),
                (((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_TAIL_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // Assert not trying to evict from empty AVL queue.
        assert!(tail_list_node_id != (NIL as u64), E_EVICT_EMPTY);
        // Determine if AVL queue is ascending.
        let ascending = order_bit == BIT_FLAG_ASCENDING;
        // Assert not trying to evict when new entry would become tail
        // (assert ascending and insertion key is less than tail key, or
        // descending and insertion key is greater than tail key).
        assert!(((ascending && (key < tail_key)) ||
            (!ascending && (key > tail_key))), E_EVICT_NEW_TAIL);
        // Immutably borrow tail list node.
        let tail_list_node_ref = table_with_length::borrow(
            &avlq_ref_mut.list_nodes, tail_list_node_id);
        // Get virtual next field from node.
        let next = ((tail_list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
            (tail_list_node_ref.next_lsbs as u64);
        // Get tree node ID encoded in next field.
        let tail_tree_node_id = next & (HI_NODE_ID as u64);
        let tail_access_key = tail_key | // Get tail access key.
            ((order_bit as u64) << SHIFT_ACCESS_SORT_ORDER) |
            ((tail_list_node_id) << SHIFT_ACCESS_LIST_NODE_ID) |
            ((tail_tree_node_id) << SHIFT_ACCESS_TREE_NODE_ID);
        // Insert new key-value insertion pair, storing access key.
        let new_access_key = insert(avlq_ref_mut, key, value);
        // Remove tail access key, storing insertion value.
        let tail_value = remove(avlq_ref_mut, tail_access_key);
        (new_access_key, tail_access_key, tail_value)
    }

    /// Return `true` if given AVL queue has ascending sort order.
    ///
    /// # Testing
    ///
    /// * `test_is_ascending()`
    public fun is_ascending<V>(
        avlq_ref: &AVLqueue<V>
    ): bool {
        ((avlq_ref.bits >> SHIFT_SORT_ORDER) & (BIT_FLAG_ASCENDING as u128)) ==
            (BIT_FLAG_ASCENDING as u128)
    }

    /// Return `true` if ascending access key, else `false`.
    ///
    /// # Testing
    ///
    /// * `test_access_key_getters()`
    public fun is_ascending_access_key(
        access_key: u64
    ): bool {
        ((access_key >> SHIFT_ACCESS_SORT_ORDER) & (HI_BIT as u64) as u8) ==
            BIT_FLAG_ASCENDING
    }

    /// Return `true` if given AVL queue is empty.
    ///
    /// # Testing
    ///
    /// * `test_is_empty()`
    public fun is_empty<V>(
        avlq_ref: &AVLqueue<V>
    ): bool {
        ((avlq_ref.bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) ==
            (NIL as u128) // Return true if no AVL queue head.
    }

    /// Return `true` if access key corresponds to a list node at the
    /// tail of its corresponding doubly linked list, aborting for an
    /// invalid list node ID.
    ///
    /// # Testing
    ///
    /// * `test_insert()`
    /// * `test_remove_2()`
    public fun is_local_tail<V>(
        avlq_ref: &AVLqueue<V>,
        access_key: u64
    ): bool {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        let list_node_ref = // Immutably borrow corresponding list node.
            table_with_length::borrow(&avlq_ref.list_nodes, list_node_id);
        // Get virtual next field from node.
        let next = ((list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.next_lsbs as u64);
        let tree_node_flag_bit = // Get tree node flag bit.
            (((next >> SHIFT_NODE_TYPE) & (BIT_FLAG_TREE_NODE as u64)) as u8);
        // Return true if next node is flagged as a tree node.
        return (tree_node_flag_bit == BIT_FLAG_TREE_NODE)
    }

    /// Return a new AVL queue, optionally allocating inactive nodes.
    ///
    /// # Parameters
    ///
    /// * `sort_order`: `ASCENDING` or `DESCENDING`.
    /// * `n_inactive_tree_nodes`: The number of inactive tree nodes
    ///   to allocate.
    /// * `n_inactive_list_nodes`: The number of inactive list nodes
    ///   to allocate.
    ///
    /// # Returns
    ///
    /// * `AVLqueue<V>`: A new AVL queue.
    ///
    /// # Aborts
    ///
    /// * `E_TOO_MANY_TREE_NODES`: Too many tree nodes specified.
    /// * `E_TOO_MANY_LIST_NODES`: Too many list nodes specified.
    ///
    /// # Testing
    ///
    /// * `test_new_no_nodes()`
    /// * `test_new_some_nodes()`
    /// * `test_new_some_nodes_loop()`
    /// * `test_new_too_many_list_nodes()`
    /// * `test_new_too_many_tree_nodes()`
    public fun new<V: store>(
        sort_order: bool,
        n_inactive_tree_nodes: u64,
        n_inactive_list_nodes: u64,
    ): AVLqueue<V> {
        // Assert not trying to allocate too many tree nodes.
        assert!(n_inactive_tree_nodes <= N_NODES_MAX, E_TOO_MANY_TREE_NODES);
        // Assert not trying to allocate too many list nodes.
        assert!(n_inactive_list_nodes <= N_NODES_MAX, E_TOO_MANY_LIST_NODES);
        // Initialize bits field based on sort order.
        let bits = if (sort_order == DESCENDING) (NIL as u128) else
            ((BIT_FLAG_ASCENDING as u128) << SHIFT_SORT_ORDER);
        // Mask in 1-indexed node ID at top of each inactive node stack.
        bits = bits | ((n_inactive_tree_nodes as u128) << SHIFT_TREE_STACK_TOP)
            | ((n_inactive_list_nodes as u128) << SHIFT_LIST_STACK_TOP);
        // Declare empty AVL queue.
        let avlq = AVLqueue {
            bits,
            root_lsbs: NIL,
            tree_nodes: table_with_length::new(),
            list_nodes: table_with_length::new(),
            values: table::new()
        };
        // If need to allocate at least one tree node:
        if (n_inactive_tree_nodes > 0) {
            let i = 0; // Declare loop counter.
            // While nodes to allocate:
            while (i < n_inactive_tree_nodes) {
                // Add to tree nodes table a node having 1-indexed node
                // ID derived from counter, indicating next inactive
                // node in stack has ID of last allocated node (or null
                // in the case of the first loop iteration).
                table_with_length::add(
                    &mut avlq.tree_nodes, i + 1, TreeNode { bits: (i as u128) });
                i = i + 1; // Increment loop counter.
            };
        };
        // If need to allocate at least one list node:
        if (n_inactive_list_nodes > 0) {
            let i = 0; // Declare loop counter.
            // While nodes to allocate:
            while (i < n_inactive_list_nodes) {
                // Add to list nodes table a node having 1-indexed node
                // ID derived from counter, indicating next inactive
                // node in stack has ID of last allocated node (or null
                // in the case of the first loop iteration).
                table_with_length::add(&mut avlq.list_nodes, i + 1, ListNode {
                    last_msbs: 0,
                    last_lsbs: 0,
                    next_msbs: ((i >> BITS_PER_BYTE) as u8),
                    next_lsbs: ((i & HI_BYTE) as u8)
                });
                // Allocate optional insertion value entry.
                table::add(&mut avlq.values, i + 1, option::none());
                i = i + 1; // Increment loop counter.
            };
        };
        avlq // Return AVL queue.
    }

    /// Get list node ID of the next list node in AVL queue, encoded in
    /// an otherwise blank access key.
    ///
    /// This function is optimized for performance and leaves access key
    /// validity checking to calling functions.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref`: Immutable reference to AVL queue.
    /// * `access_key`: Access key containing list node ID of an active
    ///   list node, relative to which the next list node ID should be
    ///   returned.
    ///
    /// # Returns
    ///
    /// * `u64`: The list node ID of the next active list node in the
    ///   AVL queue, if there is one, encoded in an otherwise blank
    ///   access key, otherwise `NIL`.
    ///
    /// # Testing
    ///
    /// * `test_next_list_node_id_in_access_key()`
    public fun next_list_node_id_in_access_key<V>(
        avlq_ref: &AVLqueue<V>,
        access_key: u64,
    ): (
    u64
    ) {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        // Immutably borrow list node.
        let list_node_ref = table_with_length::borrow(
            &avlq_ref.list_nodes, list_node_id);
        // Get virtual next field from node.
        let next = ((list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.next_lsbs as u64);
        // Determine if next node is flagged as tree node.
        let next_is_tree = ((next >> SHIFT_NODE_TYPE) &
            (BIT_FLAG_TREE_NODE as u64)) == (BIT_FLAG_TREE_NODE as u64);
        let next_node_id = next & HI_NODE_ID; // Get next node ID.
        let target_list_node_id = if (next_is_tree) {
            let target = if (is_ascending(avlq_ref))
                SUCCESSOR else PREDECESSOR;
            let (_, target_tree_node_list_head, _) =
                traverse(avlq_ref, next_node_id, target);
            target_tree_node_list_head
        } else {
            next_node_id
        };
        (target_list_node_id << SHIFT_ACCESS_LIST_NODE_ID)
    }

    /// Return insertion value at head of AVL queue, aborting if empty.
    ///
    /// # Testing
    ///
    /// * `test_pop_head_tail()`
    public fun pop_head<V>(
        avlq_ref_mut: &mut AVLqueue<V>
    ): V {
        let (list_node_id) = ((avlq_ref_mut.bits >> SHIFT_HEAD_NODE_ID) &
            (HI_NODE_ID as u128) as u64); // Get head list node ID.
        // Immutably borrow head list node.
        let list_node_ref = table_with_length::borrow(
            &mut avlq_ref_mut.list_nodes, list_node_id);
        // Get virtual last field from node.
        let last = ((list_node_ref.last_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.last_lsbs as u64);
        // Get tree node ID encoded in last field.
        let tree_node_id = last & (HI_NODE_ID as u64);
        // Encode list node and tree node IDs in partial access key.
        let access_key = (list_node_id << SHIFT_ACCESS_LIST_NODE_ID) |
            (tree_node_id << SHIFT_ACCESS_TREE_NODE_ID);
        remove(avlq_ref_mut, access_key) // Remove from AVL queue.
    }

    /// Return insertion value at tail of AVL queue, aborting if empty.
    ///
    /// # Testing
    ///
    /// * `test_pop_head_tail()`
    public fun pop_tail<V>(
        avlq_ref_mut: &mut AVLqueue<V>
    ): V {
        let (list_node_id) = ((avlq_ref_mut.bits >> SHIFT_TAIL_NODE_ID) &
            (HI_NODE_ID as u128) as u64); // Get tail list node ID.
        // Immutably borrow tail list node.
        let list_node_ref = table_with_length::borrow(
            &mut avlq_ref_mut.list_nodes, list_node_id);
        // Get virtual next field from node.
        let next = ((list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.next_lsbs as u64);
        // Get tree node ID encoded in next field.
        let tree_node_id = next & (HI_NODE_ID as u64);
        // Encode list node and tree node IDs in partial access key.
        let access_key = (list_node_id << SHIFT_ACCESS_LIST_NODE_ID) |
            (tree_node_id << SHIFT_ACCESS_TREE_NODE_ID);
        remove(avlq_ref_mut, access_key) // Remove from AVL queue.
    }

    /// Remove node having given access key, return insertion value.
    ///
    /// Update AVL queue head, tail, root fields as needed.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `access_key`: Access key returned by `insert()`.
    ///
    /// # Assumptions
    ///
    /// * Provided access key corresponds to a valid list node in the
    ///   given AVL queue.
    ///
    /// # Reference diagram
    ///
    /// Consider the following AVL queue:
    ///
    /// >        2 [3 -> 4]
    /// >       /
    /// >      1 [5 -> 6]
    ///
    /// ## Case 1 (ascending head updates)
    ///
    /// * Ascending AVL queue.
    /// * Remove insertion value 5, updating AVL queue head to node
    ///   having insertion value 6.
    /// * Remove insertion value 6, updating AVL queue head to node
    ///   having insertion value 3.
    ///
    /// ## Case 2 (ascending tail updates)
    ///
    /// * Ascending AVL queue.
    /// * Remove insertion value 4, updating AVL queue tail to node
    ///   having insertion value 3.
    /// * Remove insertion value 3, updating AVL queue tail to node
    ///   having insertion value 6.
    ///
    /// ## Case 3 (descending head updates)
    ///
    /// * Descending AVL queue.
    /// * Remove insertion value 3, updating AVL queue head to node
    ///   having insertion value 4.
    /// * Remove insertion value 4, updating AVL queue head to node
    ///   having insertion value 5.
    ///
    /// ## Case 4 (descending tail updates)
    ///
    /// * Descending AVL queue.
    /// * Remove insertion value 6, updating AVL queue tail to node
    ///   having insertion value 5.
    /// * Remove insertion value 5, updating AVL queue tail to node
    ///   having insertion value 4.
    ///
    /// # Testing
    ///
    /// * `test_remove_mid_list()` tests no modification to doubly
    ///   linked list or tail.
    /// * `test_remove_1()`, `test_remove_3()`, and `test_remove_root()`
    ///   test updates to AVL queue head.
    /// * `test_remove_2()`, `test_remove_4()`, and `test_remove_root()`
    ///   test updates to AVL queue tail.
    /// * `test_remove_1()`, `test_remove_2()`, `test_remove_3()`,
    ///   `test_remove_4()`, and `test_remove_root()` test a doubly
    ///   linked list head and tail modified, and a tree node removed.
    public fun remove<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        access_key: u64
    ): V {
        let list_node_id = // Extract list node ID from access key.
            (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID;
        // Remove list node, storing insertion value, optional new list
        // head, and optional new list tail.
        let (value, new_list_head_option, new_list_tail_option) =
            remove_list_node(avlq_ref_mut, list_node_id);
        // Check if doubly linked list head modified.
        let list_head_modified = option::is_some(&new_list_head_option);
        // Check if doubly linked list tail modified.
        let list_tail_modified = option::is_some(&new_list_tail_option);
        // If doubly linked list head or tail modified:
        if (list_head_modified || list_tail_modified) {
            let bits = avlq_ref_mut.bits; // Get AVL queue bits.
            // Get AVL queue head and tail node IDs, sort order bit.
            let (avlq_head_node_id, avlq_tail_node_id, order_bit) = (
                (((bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8));
            // Determine if AVL queue head, tail were modified.
            let (avlq_head_modified, avlq_tail_modified) =
                ((avlq_head_node_id == list_node_id),
                    (avlq_tail_node_id == list_node_id));
            // Determine if ascending AVL queue.
            let ascending = order_bit == BIT_FLAG_ASCENDING;
            let tree_node_id = // Get tree node ID from access key.
                (access_key >> SHIFT_ACCESS_TREE_NODE_ID) & HI_NODE_ID;
            // If AVL queue head modified, update accordingly.
            if (avlq_head_modified) remove_update_head(
                avlq_ref_mut, *option::borrow(&new_list_head_option),
                ascending, tree_node_id);
            // If AVL queue tail modified, update accordingly.
            if (avlq_tail_modified) remove_update_tail(
                avlq_ref_mut, *option::borrow(&new_list_tail_option),
                ascending, tree_node_id);
            // If list head and tail both modified, then just removed
            // the sole list node in a tree node, so remove tree node:
            if (list_head_modified && list_tail_modified)
                remove_tree_node(avlq_ref_mut, tree_node_id);
        };
        value // Return insertion value.
    }

    /// Return `true` if inserting `key` would update AVL queue head.
    ///
    /// # Aborts
    ///
    /// * `E_INSERTION_KEY_TOO_LARGE`: Insertion key is too large.
    ///
    /// # Testing
    ///
    /// * `test_would_update_head_tail()`
    /// * `test_would_update_head_too_big()`
    public fun would_update_head<V>(
        avlq_ref: &AVLqueue<V>,
        key: u64
    ): bool {
        // Assert insertion key is not too many bits.
        assert!(key <= HI_INSERTION_KEY, E_INSERTION_KEY_TOO_LARGE);
        let bits = avlq_ref.bits; // Get AVL queue field bits.
        // Extract relevant fields.
        let (order_bit, head_node_id, head_key) =
            ((((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8),
                (((bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_HEAD_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // Determine if AVL queue is ascending.
        let ascending = order_bit == BIT_FLAG_ASCENDING;
        // Return true if empty AVL queue with no head node.
        if (head_node_id == (NIL as u64)) return true;
        // Return true if ascending and key less than head key, or
        // descending and key greater than head key.
        return ((ascending && (key < head_key)) ||
            (!ascending && (key > head_key)))
    }

    /// Return `true` if inserting `key` would update AVL queue tail.
    ///
    /// # Aborts
    ///
    /// * `E_INSERTION_KEY_TOO_LARGE`: Insertion key is too large.
    ///
    /// # Testing
    ///
    /// * `test_would_update_head_tail()`
    /// * `test_would_update_tail_too_big()`
    public fun would_update_tail<V>(
        avlq_ref: &AVLqueue<V>,
        key: u64
    ): bool {
        // Assert insertion key is not too many bits.
        assert!(key <= HI_INSERTION_KEY, E_INSERTION_KEY_TOO_LARGE);
        let bits = avlq_ref.bits; // Get AVL queue field bits.
        // Extract relevant fields.
        let (order_bit, tail_node_id, tail_key) =
            ((((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8),
                (((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_TAIL_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // Determine if AVL queue is ascending.
        let ascending = order_bit == BIT_FLAG_ASCENDING;
        // Return true if empty AVL queue with no tail node.
        if (tail_node_id == (NIL as u64)) return true;
        // Return true if ascending and key greater than or equal to
        // tail key, or descending and key less than or equal to tail
        // key.
        return ((ascending && (key >= tail_key)) ||
            (!ascending && (key <= tail_key)))
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Check head and tail of AVL queue during insertion.
    ///
    /// Update fields as needed based on sort order.
    ///
    /// Inner function for `insert()`.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `key`: Insertion key just inserted.
    /// * `list_node_id`: ID of list node just inserted.
    ///
    /// # Testing
    ///
    /// * `test_insert_check_head_tail_ascending()`
    /// * `test_insert_check_head_tail_descending()`
    fun insert_check_head_tail<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64,
        list_node_id: u64
    ) {
        let bits = avlq_ref_mut.bits; // Get AVL queue field bits.
        // Extract relevant fields.
        let (order_bit, head_node_id, head_key, tail_node_id, tail_key) =
            ((((bits >> SHIFT_SORT_ORDER) & (HI_BIT as u128)) as u8),
                (((bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_HEAD_KEY) & (HI_INSERTION_KEY as u128)) as u64),
                (((bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_TAIL_KEY) & (HI_INSERTION_KEY as u128)) as u64));
        // Determine if AVL queue is ascending.
        let ascending = order_bit == BIT_FLAG_ASCENDING;
        let reassign_head = false; // Assume not reassigning head.
        if (head_node_id == (NIL as u64)) {
            // If no head node:
            reassign_head = true; // Mark should reassign head.
        } else {
            // Otherwise if AVL queue already had head:
            // If ascending AVL queue and insertion key less than head
            // key,
            if ((ascending && (key < head_key)) ||
                // Or if descending AVL queue and insertion key greater than
                // head key, mark should reassign head.
                (!ascending && (key > head_key))) reassign_head = true;
        };
        // Reassign bits for head key and node ID if needed:
        if (reassign_head) avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_HEAD_NODE_ID) |
                ((HI_INSERTION_KEY as u128) << SHIFT_HEAD_KEY))) |
            // Mask in new bits.
            ((list_node_id as u128) << SHIFT_HEAD_NODE_ID) |
            ((key as u128) << SHIFT_HEAD_KEY);
        let reassign_tail = false; // Assume not reassigning tail.
        if (tail_node_id == (NIL as u64)) {
            // If no tail node:
            reassign_tail = true; // Mark should reassign tail.
        } else {
            // Otherwise if AVL queue already had tail:
            // If ascending AVL queue and insertion key greater than or
            // equal to tail key,
            if ((ascending && (key >= tail_key)) ||
                // Or if descending AVL queue and insertion key less than or
                // equal to tail key, mark should reassign tail.
                (!ascending && (key <= tail_key))) reassign_tail = true;
        };
        // Reassign bits for tail key and node ID if needed:
        if (reassign_tail) avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_TAIL_NODE_ID) |
                ((HI_INSERTION_KEY as u128) << SHIFT_TAIL_KEY))) |
            // Mask in new bits.
            ((list_node_id as u128) << SHIFT_TAIL_NODE_ID) |
            ((key as u128) << SHIFT_TAIL_KEY);
    }

    /// Insert a list node and return its node ID.
    ///
    /// Inner function for `insert()`.
    ///
    /// In the case of inserting a list node to a doubly linked list in
    /// an existing tree node, known as the "anchor tree node", the list
    /// node becomes the new list tail.
    ///
    /// In the other case of inserting a "solo node" as the sole list
    /// node in a doubly linked list in a new tree leaf, the list node
    /// becomes the head and tail of the new list.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `anchor_tree_node_id`: Node ID of anchor tree node, `NIL` if
    ///   inserting a list node as the sole list node in a new tree
    ///   node.
    /// * `value`: Insertion value for list node to insert.
    ///
    /// # Returns
    ///
    /// * `u64`: Node ID of inserted list node.
    ///
    /// # Testing
    ///
    /// * `test_insert_list_node_not_solo()`
    /// * `test_insert_list_node_solo()`
    fun insert_list_node<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        anchor_tree_node_id: u64,
        value: V
    ): u64 {
        let (last, next) = // Get virtual last and next fields for node.
            insert_list_node_get_last_next(avlq_ref_mut, anchor_tree_node_id);
        let list_node_id = // Assign fields, store inserted node ID.
            insert_list_node_assign_fields(avlq_ref_mut, last, next, value);
        // If inserting a new list tail that is not solo:
        if (anchor_tree_node_id != (NIL as u64)) {
            // Mutably borrow tree nodes table.
            let tree_nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
            // Mutably borrow list nodes table.
            let list_nodes_ref_mut = &mut avlq_ref_mut.list_nodes;
            let last_node_ref_mut = // Mutably borrow old tail.
                table_with_length::borrow_mut(list_nodes_ref_mut, last);
            last_node_ref_mut.next_msbs = // Reassign its next MSBs.
                ((list_node_id >> BITS_PER_BYTE) as u8);
            // Reassign its next LSBs to those of inserted list node.
            last_node_ref_mut.next_lsbs = ((list_node_id & HI_BYTE) as u8);
            // Mutably borrow anchor tree node.
            let anchor_node_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, anchor_tree_node_id);
            // Reassign bits for list tail node:
            anchor_node_ref_mut.bits = anchor_node_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_LIST_TAIL)) |
                // Mask in new bits.
                ((list_node_id as u128) << SHIFT_LIST_TAIL);
        };
        list_node_id // Return inserted list node ID.
    }

    /// Assign fields when inserting a list node.
    ///
    /// Inner function for `insert_list_node()`.
    ///
    /// If inactive list node stack is empty, allocate a new list node,
    /// otherwise pop one off the inactive stack.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref`: Immutable reference to AVL queue.
    /// * `last`: Virtual last field from
    ///   `insert_list_node_get_last_next()`.
    /// * `next`: Virtual next field from
    ///   `insert_list_node_get_last_next()`.
    /// * `value`: Insertion value.
    ///
    /// # Returns
    ///
    /// * `u64`: Node ID of inserted list node.
    ///
    /// # Aborts
    ///
    /// `E_TOO_MANY_LIST_NODES`: Too many list nodes allocated.
    ///
    /// # Testing
    ///
    /// * `test_insert_list_node_assign_fields_allocate()`
    /// * `test_insert_list_node_assign_fields_stacked()`
    /// * `test_insert_too_many_list_nodes()`
    fun insert_list_node_assign_fields<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        last: u64,
        next: u64,
        value: V
    ): u64 {
        // Mutably borrow list nodes table.
        let list_nodes_ref_mut = &mut avlq_ref_mut.list_nodes;
        // Mutably borrow insertion values table.
        let values_ref_mut = &mut avlq_ref_mut.values;
        // Split last and next arguments into byte fields.
        let (last_msbs, last_lsbs, next_msbs, next_lsbs) = (
            ((last >> BITS_PER_BYTE) as u8), ((last & HI_BYTE) as u8),
            ((next >> BITS_PER_BYTE) as u8), ((next & HI_BYTE) as u8));
        // Get top of inactive list nodes stack.
        let list_node_id = (((avlq_ref_mut.bits >> SHIFT_LIST_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64);
        // If will need to allocate a new list node:
        if (list_node_id == (NIL as u64)) {
            // Get new 1-indexed list node ID.
            list_node_id = table_with_length::length(list_nodes_ref_mut) + 1;
            assert!(// Verify list nodes not over-allocated.
                list_node_id <= N_NODES_MAX, E_TOO_MANY_LIST_NODES);
            // Allocate a new list node with given fields.
            table_with_length::add(list_nodes_ref_mut, list_node_id, ListNode {
                last_msbs, last_lsbs, next_msbs, next_lsbs
            });
            // Allocate a new list node value option.
            table::add(values_ref_mut, list_node_id, option::some(value));
        } else {
            // If can pop inactive node off stack:
            // Mutably borrow inactive node at top of stack.
            let node_ref_mut = table_with_length::borrow_mut(
                list_nodes_ref_mut, list_node_id);
            let new_list_stack_top = // Get new list stack top node ID.
                ((node_ref_mut.next_msbs as u128) << BITS_PER_BYTE) |
                    (node_ref_mut.next_lsbs as u128);
            // Reassign bits for inactive list node stack top:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_LIST_STACK_TOP)) |
                // Mask in new bits.
                (new_list_stack_top << SHIFT_LIST_STACK_TOP);
            node_ref_mut.last_msbs = last_msbs; // Reassign last MSBs.
            node_ref_mut.last_lsbs = last_lsbs; // Reassign last LSBs.
            node_ref_mut.next_msbs = next_msbs; // Reassign next MSBs.
            node_ref_mut.next_lsbs = next_lsbs; // Reassign next LSBs.
            // Mutably borrow empty value option for node ID.
            let value_option_ref_mut =
                table::borrow_mut(values_ref_mut, list_node_id);
            // Fill the empty value option with the insertion value.
            option::fill(value_option_ref_mut, value);
        };
        list_node_id // Return list node ID.
    }

    /// Get virtual last and next fields when inserting a list node.
    ///
    /// Inner function for `insert_list_node()`.
    ///
    /// If inserted list node will be the only list node in a doubly
    /// linked list, a "solo list node", then it will have to indicate
    /// for next and last node IDs a new tree node, which will be
    /// inserted via a call to `insert_tree_node()` from within
    /// `insert()`. This only happens after `insert()` calls
    /// `insert_list_node()` and `insert_list_node()` calls
    /// `insert_list_node_assign_fields()`, which verifies that the
    /// number of allocated list nodes does not exceed the maximum
    /// permissible amount.
    ///
    /// Since the number of active tree nodes is always less than or
    /// equal to the number of active list nodes, it is thus not
    /// necessary to check on the number of allocated tree nodes here
    /// when getting a new 1-indexed tree node ID: a list node
    /// allocation violation precedes a tree node allocation violation,
    /// and `insert_list_node_assign_fields()` already checks the
    /// number of allocated list nodes during insertion (since the
    /// inactive node stack is emptied before allocating new nodes, this
    /// means that if the number of allocated list nodes is valid, then
    /// the number of allocated tree nodes is also valid).
    ///
    /// # Parameters
    ///
    /// * `avlq_ref`: Immutable reference to AVL queue.
    /// * `anchor_tree_node_id`: Node ID of anchor tree node, `NIL` if
    ///   inserting a solo list node.
    ///
    /// # Returns
    ///
    /// * `u64`: Virtual last field of inserted list node.
    /// * `u64`: Virtual next field of inserted list node.
    ///
    /// # Testing
    ///
    /// * `test_insert_list_node_get_last_next_new_tail()`
    /// * `test_insert_list_node_get_last_next_solo_allocate()`
    /// * `test_insert_list_node_get_last_next_solo_stacked()`
    fun insert_list_node_get_last_next<V>(
        avlq_ref: &AVLqueue<V>,
        anchor_tree_node_id: u64,
    ): (
        u64,
        u64
    ) {
        // Declare bitmask for flagging a tree node.
        let is_tree_node = ((BIT_FLAG_TREE_NODE as u64) << SHIFT_NODE_TYPE);
        // Immutably borrow tree nodes table.
        let tree_nodes_ref = &avlq_ref.tree_nodes;
        let last; // Declare virtual last field for inserted list node.
        // If inserting a solo list node:
        if (anchor_tree_node_id == (NIL as u64)) {
            // Get top of inactive tree nodes stack.
            anchor_tree_node_id = (((avlq_ref.bits >> SHIFT_TREE_STACK_TOP) &
                (HI_NODE_ID as u128)) as u64);
            // If will need to allocate a new tree node, get new
            // 1-indexed tree node ID.
            if (anchor_tree_node_id == (NIL as u64)) anchor_tree_node_id =
                table_with_length::length(tree_nodes_ref) + 1;
            // Set virtual last field as flagged anchor tree node ID.
            last = anchor_tree_node_id | is_tree_node;
        } else {
            // If not inserting a solo list node:
            // Immutably borrow anchor tree node.
            let anchor_node_ref = table_with_length::borrow(
                tree_nodes_ref, anchor_tree_node_id);
            // Set virtual last field as anchor node list tail.
            last = (((anchor_node_ref.bits >> SHIFT_LIST_TAIL) &
                (HI_NODE_ID as u128)) as u64);
        };
        // Return virtual last field per above, and virtual next field
        // as flagged anchor tree node ID.
        (last, (anchor_tree_node_id | is_tree_node))
    }

    /// Insert a tree node and return its node ID.
    ///
    /// Inner function for `insert()`.
    ///
    /// If inactive tree node stack is empty, allocate a new tree node,
    /// otherwise pop one off the inactive stack.
    ///
    /// Should only be called when `insert_list_node()` inserts the
    /// sole list node in new AVL tree node, thus checking the number
    /// of allocated list nodes per `insert_list_node_assign_fields()`.
    /// As discussed in `insert_list_node_get_last_next()`, this check
    /// verifies the number of allocated tree nodes, since the number
    /// of active tree nodes is less than or equal to the number of
    /// active list nodes.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `key`: Insertion key for inserted node.
    /// * `parent`: Node ID of parent to inserted node, `NIL` when
    ///   inserted node is to become root.
    /// * `solo_node_id`: Node ID of sole list node in tree node's
    ///   doubly linked list.
    /// * `new_leaf_side`: None if inserted node is root, `LEFT` if
    ///   inserted node is left child of its parent, and `RIGHT` if
    ///   inserted node is right child of its parent.
    ///
    /// # Returns
    ///
    /// * `u64`: Node ID of inserted tree node.
    ///
    /// # Assumptions
    ///
    /// * Node is a leaf in the AVL tree and has a single list node in
    ///   its doubly linked list.
    /// * The number of allocated tree nodes has already been checked
    ///   via `insert_list_node_get_last_next()`.
    /// * All `u64` fields correspond to valid node IDs.
    ///
    /// # Testing
    ///
    /// * `test_insert_tree_node_empty()`
    /// * `test_insert_tree_node_stacked()`
    fun insert_tree_node<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64,
        parent: u64,
        solo_node_id: u64,
        new_leaf_side: Option<bool>
    ): u64 {
        // Pack field bits.
        let bits = ((key as u128) << SHIFT_INSERTION_KEY) |
            ((parent as u128) << SHIFT_PARENT) |
            ((solo_node_id as u128) << SHIFT_LIST_HEAD) |
            ((solo_node_id as u128) << SHIFT_LIST_TAIL);
        // Get top of inactive tree nodes stack.
        let tree_node_id = (((avlq_ref_mut.bits >> SHIFT_TREE_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64);
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // If need to allocate new tree node:
        if (tree_node_id == (NIL as u64)) {
            // Get new 1-indexed tree node ID.
            tree_node_id = table_with_length::length(tree_nodes_ref_mut) + 1;
            table_with_length::add(// Allocate new packed tree node.
                tree_nodes_ref_mut, tree_node_id, TreeNode { bits })
        } else {
            // If can pop inactive node off stack:
            // Mutably borrow inactive node at top of stack.
            let node_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, tree_node_id);
            // Get new inactive tree nodes stack top node ID.
            let new_tree_stack_top = node_ref_mut.bits & (HI_NODE_ID as u128);
            // Reassign bits for inactive tree node stack top:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_TREE_STACK_TOP)) |
                // Mask in new bits.
                (new_tree_stack_top << SHIFT_TREE_STACK_TOP);
            node_ref_mut.bits = bits; // Reassign inserted node bits.
        };
        insert_tree_node_update_parent_edge(// Update parent edge.
            avlq_ref_mut, tree_node_id, parent, new_leaf_side);
        tree_node_id // Return inserted tree node ID.
    }

    /// Update the parent edge for a tree node just inserted.
    ///
    /// Inner function for `insert_tree_node()`.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `tree_node_id`: Node ID of tree node just inserted in
    ///   `insert_tree_node()`.
    /// * `parent`: Node ID of parent to inserted node, `NIL` when
    ///   inserted node is root.
    /// * `new_leaf_side`: None if inserted node is root, `LEFT` if
    ///   inserted node is left child of its parent, and `RIGHT` if
    ///   inserted node is right child of its parent.
    ///
    /// # Testing
    ///
    /// * `test_insert_tree_node_update_parent_edge_left()`
    /// * `test_insert_tree_node_update_parent_edge_right()`
    /// * `test_insert_tree_node_update_parent_edge_root()`
    fun insert_tree_node_update_parent_edge<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        tree_node_id: u64,
        parent: u64,
        new_leaf_side: Option<bool>
    ) {
        if (option::is_none(&new_leaf_side)) {
            // If inserting root:
            // Reassign bits for root MSBs:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID >> BITS_PER_BYTE) as u128)) |
                // Mask in new bits.
                ((tree_node_id as u128) >> BITS_PER_BYTE);
            // Set root LSBs.
            avlq_ref_mut.root_lsbs = ((tree_node_id & HI_BYTE) as u8);
        } else {
            // If inserting child to existing node:
            // Mutably borrow tree nodes table.
            let tree_nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
            // Mutably borrow parent.
            let parent_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, parent);
            // Determine if inserting left child.
            let left_child = *option::borrow(&new_leaf_side) == LEFT;
            // Get child node ID field shift amounts for given side;
            let child_shift = if (left_child) SHIFT_CHILD_LEFT else
                SHIFT_CHILD_RIGHT;
            // Reassign bits for child field on given side.
            parent_ref_mut.bits = parent_ref_mut.bits &
                // Clear out all bits via mask unset at relevant bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << child_shift)) |
                // Mask in new bits.
                ((tree_node_id as u128) << child_shift);
        };
    }

    /// Remove list node for given access key, return insertion value.
    ///
    /// Inner function for `remove()`.
    ///
    /// Updates last and next nodes in doubly linked list, optionally
    /// updating head or tail field in corresponding tree node if list
    /// node was head or tail of doubly linked list. Does not modify
    /// corresponding tree node if list node was sole node in doubly
    /// linked list.
    ///
    /// Pushes inactive list node onto inactive list nodes stack.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `list_node_id`: List node ID of node to remove.
    ///
    /// # Returns
    ///
    /// * `V`: Corresponding insertion value.
    /// * `Option<u64>`: New list head node ID, if any, with `NIL`
    ///   indicating that corresponding doubly linked list has been
    ///   cleared out.
    /// * `Option<u64>`: New list tail node ID, if any, with `NIL`
    ///   indicating that corresponding doubly linked list has been
    ///   cleared out.
    ///
    /// # Testing
    ///
    /// * `test_remove_list_node()`
    fun remove_list_node<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        list_node_id: u64
    ): (
        V,
        Option<u64>,
        Option<u64>
    ) {
        // Mutably borrow list nodes table.
        let list_nodes_ref_mut = &mut avlq_ref_mut.list_nodes;
        let list_node_ref_mut = // Mutably borrow list node.
            table_with_length::borrow_mut(list_nodes_ref_mut, list_node_id);
        // Get virtual last field.
        let last = ((list_node_ref_mut.last_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref_mut.last_lsbs as u64);
        // Get virtual next field.
        let next = ((list_node_ref_mut.next_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref_mut.next_lsbs as u64);
        // Determine if last node is flagged as tree node.
        let last_is_tree = ((last >> SHIFT_NODE_TYPE) &
            (BIT_FLAG_TREE_NODE as u64)) == (BIT_FLAG_TREE_NODE as u64);
        // Determine if next node is flagged as tree node.
        let next_is_tree = ((next >> SHIFT_NODE_TYPE) &
            (BIT_FLAG_TREE_NODE as u64)) == (BIT_FLAG_TREE_NODE as u64);
        let last_node_id = last & HI_NODE_ID; // Get last node ID.
        let next_node_id = next & HI_NODE_ID; // Get next node ID.
        // Get inactive list nodes stack top.
        let list_top = (((avlq_ref_mut.bits >> SHIFT_LIST_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64);
        list_node_ref_mut.last_msbs = 0; // Clear node's last MSBs.
        list_node_ref_mut.last_lsbs = 0; // Clear node's last LSBs.
        // Set node's next MSBs to those of inactive stack top.
        list_node_ref_mut.next_msbs = ((list_top >> BITS_PER_BYTE) as u8);
        // Set node's next LSBs to those of inactive stack top.
        list_node_ref_mut.next_lsbs = ((list_top & (HI_BYTE as u64)) as u8);
        // Reassign bits for inactive list node stack top:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_LIST_STACK_TOP)) |
            // Mask in new bits.
            ((list_node_id as u128) << SHIFT_LIST_STACK_TOP);
        // Update node edges, storing optional new head and tail.
        let (new_head, new_tail) = remove_list_node_update_edges(
            avlq_ref_mut, last, next, last_is_tree, next_is_tree, last_node_id,
            next_node_id);
        // Mutably borrow insertion values table.
        let values_ref_mut = &mut avlq_ref_mut.values;
        let value = option::extract(// Extract insertion value.
            table::borrow_mut(values_ref_mut, list_node_id));
        // Return insertion value, optional new head, optional new tail.
        (value, new_head, new_tail)
    }

    /// Update node edges when removing a list node.
    ///
    /// Inner function for `remove_list_node()`.
    ///
    /// Update last and next edges relative to removed list node,
    /// returning optional new list head and tail list node IDs. If
    /// removed list node was sole node in doubly linked list, does not
    /// modify corresponding tree node.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `last`: Virtual last field from removed list node.
    /// * `next`: Virtual next field from removed list node.
    /// * `last_is_tree`: `true` if last node is flagged as tree node.
    /// * `next_is_tree`: `true` if next node is flagged as tree node.
    /// * `last_node_id`: Node ID of last node.
    /// * `next_node_id`: Node ID of next node.
    ///
    /// # Returns
    ///
    /// * `Option<u64>`: New list head node ID, if any, with `NIL`
    ///   indicating that corresponding doubly linked list has been
    ///   cleared out.
    /// * `Option<u64>`: New list tail node ID, if any, with `NIL`
    ///   indicating that corresponding doubly linked list has been
    ///   cleared out.
    ///
    /// # Testing
    ///
    /// * `test_remove_list_node()`
    fun remove_list_node_update_edges<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        last: u64,
        next: u64,
        last_is_tree: bool,
        next_is_tree: bool,
        last_node_id: u64,
        next_node_id: u64
    ): (
        Option<u64>,
        Option<u64>
    ) {
        // If node was sole list node in doubly linked list, return that
        // the doubly linked list has been cleared out.
        if (last_is_tree && next_is_tree) return
            (option::some((NIL as u64)), option::some((NIL as u64)));
        // Otherwise, assume no new list head or tail.
        let (new_head, new_tail) = (option::none(), option::none());
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // Mutably borrow list nodes table.
        let list_nodes_ref_mut = &mut avlq_ref_mut.list_nodes;
        if (last_is_tree) {
            // If removed node was list head:
            // Mutably borrow corresponding tree node.
            let tree_node_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, last_node_id);
            // Reassign bits for list head to next node ID:
            tree_node_ref_mut.bits = tree_node_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_LIST_HEAD)) |
                // Mask in new bits.
                ((next_node_id as u128) << SHIFT_LIST_HEAD);
            new_head = option::some(next_node_id); // Flag new head.
        } else {
            // If node was not list head:
            // Mutably borrow last list node.
            let list_node_ref_mut = table_with_length::borrow_mut(
                list_nodes_ref_mut, last_node_id);
            // Set node's next MSBs to those of virtual next field.
            list_node_ref_mut.next_msbs = ((next >> BITS_PER_BYTE) as u8);
            // Set node's next LSBs to those of virtual next field.
            list_node_ref_mut.next_lsbs = ((next & (HI_BYTE as u64)) as u8);
        };
        if (next_is_tree) {
            // If removed node was list tail:
            // Mutably borrow corresponding tree node.
            let tree_node_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, next_node_id);
            // Reassign bits for list tail to last node ID:
            tree_node_ref_mut.bits = tree_node_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_LIST_TAIL)) |
                // Mask in new bits.
                ((last_node_id as u128) << SHIFT_LIST_TAIL);
            new_tail = option::some(last_node_id); // Flag new tail.
        } else {
            // If node was not list tail:
            // Mutably borrow next list node.
            let list_node_ref_mut = table_with_length::borrow_mut(
                list_nodes_ref_mut, next_node_id);
            // Set node's last MSBs to those of virtual next field.
            list_node_ref_mut.last_msbs = ((last >> BITS_PER_BYTE) as u8);
            // Set node's last LSBs to those of virtual next field.
            list_node_ref_mut.last_lsbs = ((last & (HI_BYTE as u64)) as u8);
        };
        (new_head, new_tail) // Return optional new head and tail.
    }

    /// Remove tree node from an AVL queue.
    ///
    /// Inner function for `remove()`.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of node to remove.
    ///
    /// Here, node x refers to the node to remove from the tree. Node
    /// x may have a parent or may be the tree root, and may have 0, 1,
    /// or 2 children.
    ///
    /// >        |
    /// >        x
    /// >       / \
    ///
    /// # Case 1
    ///
    /// Node x has no children. Here, the parent to node x gets updated
    /// to have a null subtree as its child on the side that node x used
    /// to be a child at. If node x has no parent the tree is completely
    /// cleared out and no retrace takes place, otherwise a decrement
    /// retrace starts from node x's pre-removal parent on the side that
    /// node x used to be a child at.
    ///
    /// # Case 2
    ///
    /// Node x has a single child node. Here, the parent to node x gets
    /// updated to have node x's sole child as its child on the side
    /// that node x used to be a child at. If node x has no parent then
    /// the child becomes the root of the tree and no retrace takes
    /// place, otherwise a decrement retrace starts from node x's
    /// pre-removal parent on the side that node x used to be a child
    /// at.
    ///
    /// ## Left child
    ///
    /// Pre-removal:
    ///
    /// >       |
    /// >       x
    /// >      /
    /// >     l
    ///
    /// Post-removal:
    ///
    /// >     |
    /// >     l
    ///
    /// ## Right child
    ///
    /// Pre-removal:
    ///
    /// >     |
    /// >     x
    /// >      \
    /// >       r
    ///
    /// Post-removal:
    ///
    /// >     |
    /// >     r
    ///
    /// # Case 3
    ///
    /// Node x has two children. Handled by
    /// `remove_tree_node_with_children()`.
    ///
    /// # Testing
    ///
    /// * `test_remove_root_twice()` and `test_rotate_left_2()` test
    ///   case 1.
    /// * `test_rotate_right_left_2()` tests case 2 left child variant.
    /// * `test_remove_root_twice()` and `test_rotate_left_right_1()`
    ///   test case 2 right child variant.
    /// * `test_remove_children_1()`, `test_remove_children_2()`, and
    ///   `test_remove_children_3()` test case 3.
    ///
    /// See tests for more information on their corresponding reference
    /// diagrams.
    fun remove_tree_node<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64
    ) {
        let node_x_ref = // Immutably borrow node x.
            table_with_length::borrow(&mut avlq_ref_mut.tree_nodes, node_x_id);
        let bits = node_x_ref.bits; // Get node x bits.
        // Get node x's left height, right height, parent, and children
        // fields.
        let (node_x_height_left, node_x_height_right, node_x_parent,
            node_x_child_left, node_x_child_right) =
            ((((bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_PARENT) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_CHILD_LEFT) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_CHILD_RIGHT) & (HI_NODE_ID as u128)) as u64));
        // Determine if node x has left child.
        let has_child_left = node_x_child_left != (NIL as u64);
        // Determine if node x has right child.
        let has_child_right = node_x_child_right != (NIL as u64);
        // Assume case 1: node x is leaf node replaced by null subtree,
        // potentially requiring decrement retrace on side that node x
        // was child at (retrace side reassigned later).
        let (new_subtree_root, retrace_node_id, retrace_side) =
            ((NIL as u64), node_x_parent, false);
        if ((has_child_left && !has_child_right) ||
            (!has_child_left && has_child_right)) {
            // If only 1 child:
            new_subtree_root = if (has_child_left) node_x_child_left else
                node_x_child_right; // New subtree root is the child.
            // Mutably borrow child.
            let child_ref_mut = table_with_length::borrow_mut(
                &mut avlq_ref_mut.tree_nodes, new_subtree_root);
            // Reassign bits for new parent field.
            child_ref_mut.bits = child_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_x_parent as u128) << SHIFT_PARENT);
        }; // Case 2 handled.
        // If node x has two children, remove node per case 3, storing
        // new subtree root, retrace node ID, and retrace side.
        if (has_child_left && has_child_right)
            (new_subtree_root, retrace_node_id, retrace_side) =
                remove_tree_node_with_children(
                    avlq_ref_mut, node_x_height_left, node_x_height_right,
                    node_x_parent, node_x_child_left, node_x_child_right);
        // Clean up parent edge, optionally retrace, push onto stack.
        remove_tree_node_follow_up(
            avlq_ref_mut, node_x_id, node_x_parent, new_subtree_root,
            retrace_node_id, retrace_side);
    }

    /// Clean up parent edge, optionally retrace, push onto stack.
    ///
    /// Inner function for `remove_tree_node()`, following up on removal
    /// of node x.
    ///
    /// Follow up on tree node removal re-ordering operations, updating
    /// parent to node x (if there is one). Retrace as needed, then push
    /// node x onto the inactive tree nodes stack.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of removed node.
    /// * `node_x_parent`: Parent field of node x before it was removed,
    ///   `NIL` if x was root.
    /// * `new_subtree_root`: New root of subtree where node x was root
    ///   pre-removal, `NIL` if node x was a leaf node.
    /// * `retrace_node_id`: Node ID to retrace from, `NIL` if node x
    ///   was at the root and had less than two children before it was
    ///   removed.
    /// * `retrace_side`: Side of decrement retrace for the case of node
    ///   x having two children, reassigned if node x was not at root
    ///   before removal and had less than two children.
    ///
    /// # Testing
    ///
    /// * `test_remove_root_twice()`, `test_remove_children_2()`, and
    ///   `test_remove_children_3()` test node x is tree root.
    /// * `test_rotate_left_2()` and `test_rotate_right_left_2()` test
    ///   node x is left child.
    /// * `test_remove_children_1()` and `test_rotate_left_right_1()`
    ///   test node x is right child.
    /// * `test_rotate_left_2()` and `test_rotate_right_left_2()` test
    ///   retrace node ID is node x's parent, for node x is left child
    ///   and has less than two children.
    /// * `test_rotate_left_right_1()` tests retrace node ID is node
    ///   x's parent, for node x is right child and has less than two
    ///   children.
    /// * `test_remove_children_1()` tests retrace node ID is not node
    ///   x's parent, for node x is not root and has two children.
    /// * `test_remove_root_twice()` tests node x as root with less than
    ///   two children, where retrace node ID is null and no retrace
    ///   needed.
    /// * Above tests vary whether the inactive tree node stack is empty
    ///   before removal, with `test_remove_root_twice()` in particular
    ///   covering both cases.
    fun remove_tree_node_follow_up<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_x_parent: u64,
        new_subtree_root: u64,
        retrace_node_id: u64,
        retrace_side: bool
    ) {
        if (node_x_parent == (NIL as u64)) {
            // If node x was tree root:
            // Reassign bits for root MSBs:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) >> BITS_PER_BYTE)) |
                // Mask in new bits.
                ((new_subtree_root as u128) >> BITS_PER_BYTE);
            avlq_ref_mut.root_lsbs = // Set AVL queue root LSBs.
                ((new_subtree_root & HI_BYTE) as u8);
        } else {
            // If node x was not root:
            // Mutably borrow node x's parent.
            let parent_ref_mut = table_with_length::borrow_mut(
                &mut avlq_ref_mut.tree_nodes, node_x_parent);
            // Get parent's left child.
            let parent_left_child = (((parent_ref_mut.bits >> SHIFT_CHILD_LEFT)
                & (HI_NODE_ID as u128)) as u64);
            // Get child shift based on node x's side as a child.
            let child_shift = if (parent_left_child == node_x_id)
                SHIFT_CHILD_LEFT else SHIFT_CHILD_RIGHT;
            // Reassign bits for new child field.
            parent_ref_mut.bits = parent_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << child_shift)) |
                // Mask in new bits.
                ((new_subtree_root as u128) << child_shift);
            // If retrace node id is node x's parent, then node x had
            // less than two children before removal, so retrace side
            // is the side on which node x was previously a child.
            if (retrace_node_id == node_x_parent) retrace_side =
                if (parent_left_child == node_x_id) LEFT else RIGHT;
        }; // Parent edge updated, retrace side assigned if needed.
        // Retrace if node x was not root having less than two children.
        if (retrace_node_id != (NIL as u64))
            retrace(avlq_ref_mut, retrace_node_id, DECREMENT, retrace_side);
        // Get inactive tree nodes stack top.
        let tree_top = (((avlq_ref_mut.bits >> SHIFT_TREE_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64);
        // Mutably borrow node x.
        let node_x_ref_mut = table_with_length::borrow_mut(
            &mut avlq_ref_mut.tree_nodes, node_x_id);
        // Set node x to indicate the next inactive tree node in stack.
        node_x_ref_mut.bits = (tree_top as u128);
        // Reassign bits for inactive tree node stack top:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_TREE_STACK_TOP)) |
            // Mask in new bits.
            ((node_x_id as u128) << SHIFT_TREE_STACK_TOP);
    }

    /// Replace node x with its predecessor in preparation for retrace.
    ///
    /// Inner function for `remove_tree_node()` in the case of removing
    /// a node with two children. Here, node x is the node to remove,
    /// having left child node l and right child node r.
    ///
    /// >       |
    /// >       x
    /// >      / \
    /// >     l   r
    ///
    /// Does not modify state of node x, which is updated later via
    /// `remove_tree_node_follow_up()`. Similarly does not modify state
    /// for parent of node x or of AVL queue root field.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_height_left`: Node x's left height.
    /// * `node_x_height_right`: Node x's right height.
    /// * `node_x_parent`: Node x's parent field.
    /// * `node_l_id`: Node ID of node x's left child.
    /// * `node_r_id`: Node ID of node x's right child.
    ///
    /// # Returns
    ///
    /// * `u64`: Node ID of new root subtree where node x was root
    ///   pre-removal.
    /// * `u64`: Node ID of node to begin decrement retrace from in
    ///   `remove_tree_node_follow_up()`.
    /// * `bool`: `LEFT` or `RIGHT`, the side on which the decrement
    ///   retrace should take place.
    ///
    /// # Predecessor is immediate child
    ///
    /// Node l does not have a right child, but has left child tree l
    /// which may or may not be empty.
    ///
    /// >           |
    /// >           x
    /// >          / \
    /// >         l   r
    /// >        /
    /// >     t_l
    ///
    /// Here, node l takes the place of node x, with node l's left
    /// height and right height set to those of node x pre-removal. Then
    /// a left decrement retrace is initiated at node l.
    ///
    /// >         |
    /// >         l
    /// >        / \
    /// >     t_l   r
    ///
    /// # Predecessor is not immediate child
    ///
    /// Node l has a right child, with node y as the maximum node in the
    /// corresponding subtree. Node y has no right child, but has as its
    /// left child tree y, which may or may not be empty. Node y may or
    /// may not have node l as its parent.
    ///
    /// >           |
    /// >           x
    /// >          / \
    /// >         l   r
    /// >        / \
    /// >     t_l   ~
    /// >            \
    /// >             y
    /// >            /
    /// >         t_y
    ///
    /// Here, node y takes the place of node x, with node y's left
    /// height and right height set to those of node x pre-removal. Tree
    /// y then takes the place of y, and a right decrement retrace is
    /// initiated at node y's pre-removal parent.
    ///
    /// >           |
    /// >           y
    /// >          / \
    /// >         l   r
    /// >        / \
    /// >     t_l   ~
    /// >            \
    /// >             t_y
    ///
    /// # Reference diagrams
    ///
    /// ## Case 1
    ///
    /// * Node x is not root.
    /// * Node x has insertion key 4.
    /// * Predecessor is node x's child.
    /// * Node l has insertion key 3.
    ///
    /// Pre-removal:
    ///
    /// >               2
    /// >              / \
    /// >             1   4 <- node x
    /// >                / \
    /// >     node l -> 3   5 <- node r
    ///
    /// Post-removal:
    ///
    /// >       2
    /// >      / \
    /// >     1   3
    /// >          \
    /// >           5
    ///
    /// ## Case 2
    ///
    /// * Node x is root.
    /// * Node x has insertion key 5.
    /// * Predecessor is not node x's child.
    /// * Predecessor is node l's child.
    /// * Node y has insertion key 4.
    /// * Subtree y is not empty.
    ///
    /// Pre-removal:
    ///
    /// >                   5 <- node x
    /// >                  / \
    /// >       node l -> 2   6 <- node r
    /// >                / \   \
    /// >     tree l -> 1   4   7
    /// >                  /
    /// >       tree y -> 3
    ///
    /// Post-removal:
    ///
    /// >         4
    /// >        / \
    /// >       2   6
    /// >      / \   \
    /// >     1   3   7
    ///
    /// ## Case 3
    ///
    /// * Node x is root.
    /// * Node x has insertion key 5.
    /// * Predecessor is not node x's child.
    /// * Predecessor is not node l's child.
    /// * Node y has insertion key 4.
    /// * Subtree y is empty.
    ///
    /// Pre-removal:
    ///
    /// >                   5 <- node x
    /// >                  / \
    /// >       node l -> 2   6 <- node r
    /// >                / \   \
    /// >     tree l -> 1   3   7
    /// >                    \
    /// >                     4 <- node y
    ///
    /// Post-removal:
    ///
    /// >         4
    /// >        / \
    /// >       2   6
    /// >      / \   \
    /// >     1   3   7
    ///
    /// # Testing
    ///
    /// * `test_remove_children_1()`
    /// * `test_remove_children_2()`
    /// * `test_remove_children_3()`
    fun remove_tree_node_with_children<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_height_left: u8,
        node_x_height_right: u8,
        node_x_parent: u64,
        node_l_id: u64,
        node_r_id: u64,
    ): (
        u64,
        u64,
        bool
    ) {
        // Declare returns.
        let (new_subtree_root, retrace_node_id, retrace_side);
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        let node_l_ref_mut = // Mutably borrow node l.
            table_with_length::borrow_mut(tree_nodes_ref_mut, node_l_id);
        let bits = node_l_ref_mut.bits; // Get node l bits.
        let node_l_child_right = // Get node l's right child field.
            (((bits >> SHIFT_CHILD_RIGHT) & (HI_NODE_ID as u128)) as u64);
        // If node l has no right child (if is immediate predecessor):
        if (node_l_child_right == (NIL as u64)) {
            // Reassign node l bits for parent, heights, right child.
            node_l_ref_mut.bits = node_l_ref_mut.bits &
                // Clear out fields via mask unset at field bits.
                (HI_128 ^ (((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                    ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                    ((HI_NODE_ID as u128) << SHIFT_PARENT) |
                    ((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT))) |
                // Mask in new bits.
                ((node_x_height_left as u128) << SHIFT_HEIGHT_LEFT) |
                ((node_x_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
                ((node_x_parent as u128) << SHIFT_PARENT) |
                ((node_r_id as u128) << SHIFT_CHILD_RIGHT);
            // Assign returns accordingly.
            (new_subtree_root, retrace_node_id, retrace_side) =
                (node_l_id, node_l_id, LEFT);
        } else {
            // If node x predecessor is in node l's right subtree:
            // Assign node l's right child as a candidate for node y.
            let node_y_id = node_l_child_right;
            let node_y_ref_mut; // Declare mutable reference to node y.
            loop {
                // Loop down node l's right subtree
                // Mutably borrow node y candidate.
                node_y_ref_mut = table_with_length::borrow_mut(
                    tree_nodes_ref_mut, node_y_id);
                let child_right = // Get candidate's right child field.
                    (((node_y_ref_mut.bits >> SHIFT_CHILD_RIGHT) &
                        (HI_NODE_ID as u128)) as u64);
                // Break if no right child, since have found node y.
                if (child_right == (NIL as u64)) break;
                // Otherwise child is candidate for new iteration.
                node_y_id = child_right;
            }; // Node y found.
            let bits = node_y_ref_mut.bits; // Get node y bits.
            // Get node y's parent ID and tree y ID.
            let (node_y_parent_id, tree_y_id) =
                ((((bits >> SHIFT_PARENT) & (HI_NODE_ID as u128)) as u64),
                    (((bits >> SHIFT_CHILD_LEFT) & (HI_NODE_ID as u128)) as u64));
            // Reassign node y bits for parent, heights, children.
            node_y_ref_mut.bits = bits &
                // Clear out fields via mask unset at field bits.
                (HI_128 ^ (((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                    ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                    ((HI_NODE_ID as u128) << SHIFT_PARENT) |
                    ((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                    ((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT))) |
                // Mask in new bits.
                ((node_x_height_left as u128) << SHIFT_HEIGHT_LEFT) |
                ((node_x_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
                ((node_x_parent as u128) << SHIFT_PARENT) |
                ((node_l_id as u128) << SHIFT_CHILD_LEFT) |
                ((node_r_id as u128) << SHIFT_CHILD_RIGHT);
            // Mutably borrow node y's parent.
            let node_y_parent_ref_mut = table_with_length::borrow_mut(
                tree_nodes_ref_mut, node_y_parent_id);
            // Reassign bits for parent's right child field:
            node_y_parent_ref_mut.bits = node_y_parent_ref_mut.bits &
                // Clear out fields via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT)) |
                // Mask in new bits.
                ((tree_y_id as u128) << SHIFT_CHILD_RIGHT);
            // Mutably borrow node l, which may be node y's parent.
            node_l_ref_mut = if (node_y_parent_id == node_l_id)
                node_y_parent_ref_mut else table_with_length::borrow_mut(
                tree_nodes_ref_mut, node_l_id);
            // Reassign bits for node l's parent field:
            node_l_ref_mut.bits = node_l_ref_mut.bits &
                // Clear out fields via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_y_id as u128) << SHIFT_PARENT);
            if (tree_y_id != (NIL as u64)) {
                // If tree y not null:
                // Mutably borrow tree y's root.
                let tree_y_ref_mut = table_with_length::borrow_mut(
                    tree_nodes_ref_mut, tree_y_id);
                // Reassign bits for corresponding parent field:
                tree_y_ref_mut.bits = tree_y_ref_mut.bits &
                    // Clear out fields via mask unset at field bits.
                    (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                    // Mask in new bits.
                    ((node_y_parent_id as u128) << SHIFT_PARENT);
            };
            // Assign returns accordingly.
            (new_subtree_root, retrace_node_id, retrace_side) =
                (node_y_id, node_y_parent_id, RIGHT);
        };
        let node_r_ref_mut = // Mutably borrow node r.
            table_with_length::borrow_mut(tree_nodes_ref_mut, node_r_id);
        // Reassign bits for node r parent field:
        node_r_ref_mut.bits = node_r_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
            // Mask in new bits.
            ((new_subtree_root as u128) << SHIFT_PARENT);
        // Return new subtree root, node ID to retrace from, side to
        // retrace on.
        (new_subtree_root, retrace_node_id, retrace_side)
    }

    /// Update AVL queue head during removal.
    ///
    /// Inner function for `remove()`, should only be called if AVL
    /// queue head is modified.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `new_list_head`: New head of corresponding doubly linked list,
    ///   `NIL` if doubly linked list is cleared out by removal.
    /// * `ascending`: `true` if ascending AVL queue, else `false`.
    /// * `tree_node_id`: Node ID of corresponding tree node.
    ///
    /// # Testing
    ///
    /// * `test_remove_1()` and `test_remove_3()` test new list head is
    ///   a list node ID.
    /// * `test_remove_1()` and `test_remove_3()` test new list head is
    ///   not a list node ID, for traversal where start node is not sole
    ///   leaf at root.
    /// * `test_remove_1()` tests ascending AVL queue.
    /// * `test_remove_3()` tests descending AVL queue.
    /// * `test_remove_root()` tests new list head is not a list node
    ///   ID, for traversal where start node is sole leaf at root.
    fun remove_update_head<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        new_list_head: u64,
        ascending: bool,
        tree_node_id: u64
    ) {
        // Declare new AVL queue head node ID.
        let new_avlq_head_node_id;
        // If new list head is a list node ID:
        if (new_list_head != (NIL as u64)) {
            // Then it becomes the new AVL queue head node ID.
            new_avlq_head_node_id = new_list_head;
            // Otherwise, if new list head is null, then just cleared out
            // sole list node in a doubly linked list:
        } else {
            // Declare target tree node to traverse to.
            let target = if (ascending) SUCCESSOR else PREDECESSOR;
            // Declare new AVL queue head insertion key.
            let new_avlq_head_insertion_key;
            // Get new AVL queue head insertion key and node ID by
            // traversing to corresponding tree node (both null if start
            // node is sole leaf at root).
            (new_avlq_head_insertion_key, new_avlq_head_node_id, _)
                    = traverse(avlq_ref_mut, tree_node_id, target);
            // Reassign bits for AVL queue head insertion key:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_INSERTION_KEY as u128) << SHIFT_HEAD_KEY)) |
                // Mask in new bits.
                ((new_avlq_head_insertion_key as u128) << SHIFT_HEAD_KEY);
        };
        // Reassign bits for AVL queue head node ID:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_HEAD_NODE_ID)) |
            // Mask in new bits.
            ((new_avlq_head_node_id as u128) << SHIFT_HEAD_NODE_ID);
    }

    /// Update AVL queue tail during removal.
    ///
    /// Inner function for `remove()`, should only be called if AVL
    /// queue tail is modified.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `new_list_tail`: New tail of corresponding doubly linked list,
    ///   `NIL` if doubly linked list is cleared out by removal.
    /// * `ascending`: `true` if ascending AVL queue, else `false`.
    /// * `tree_node_id`: Node ID of corresponding tree node.
    ///
    /// # Testing
    ///
    /// * `test_remove_2()` and `test_remove_4()` test new list tail is
    ///   a list node ID.
    /// * `test_remove_2()` and `test_remove_4()` test new list tail is
    ///   not a list node ID, for traversal where start node is not sole
    ///   leaf at root.
    /// * `test_remove_2()` tests ascending AVL queue.
    /// * `test_remove_4()` tests descending AVL queue.
    /// * `test_remove_root()` tests new list tail is not a list node
    ///   ID, for traversal where start node is sole leaf at root.
    fun remove_update_tail<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        new_list_tail: u64,
        ascending: bool,
        tree_node_id: u64
    ) {
        // Declare new AVL queue tail node ID.
        let new_avlq_tail_node_id;
        // If new list tail is a list node ID:
        if (new_list_tail != (NIL as u64)) {
            // Then it becomes the new AVL queue tail node ID.
            new_avlq_tail_node_id = new_list_tail;
            // Otherwise, if new list tail is null, then just cleared out
            // sole list node in a doubly linked list:
        } else {
            // Declare target tree node to traverse to.
            let target = if (ascending) PREDECESSOR else SUCCESSOR;
            // Declare new AVL queue tail insertion key.
            let new_avlq_tail_insertion_key;
            // Get new AVL queue tail insertion key and node ID by
            // traversing to corresponding tree node (both null if start
            // node is sole leaf at root).
            (new_avlq_tail_insertion_key, _, new_avlq_tail_node_id)
                    = traverse(avlq_ref_mut, tree_node_id, target);
            // Reassign bits for AVL queue tail insertion key:
            avlq_ref_mut.bits = avlq_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_INSERTION_KEY as u128) << SHIFT_TAIL_KEY)) |
                // Mask in new bits.
                ((new_avlq_tail_insertion_key as u128) << SHIFT_TAIL_KEY);
        };
        // Reassign bits for AVL queue tail node ID:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_TAIL_NODE_ID)) |
            // Mask in new bits.
            ((new_avlq_tail_node_id as u128) << SHIFT_TAIL_NODE_ID);
    }

    /// Retrace ancestor heights after tree node insertion or removal.
    ///
    /// Should only be called by `insert()` or
    /// `remove_tree_node_follow_up()`.
    ///
    /// When a tree node is inserted or removed, a parent-child edge is
    /// updated with corresponding node IDs for both parent and optional
    /// child. Then the corresponding change in height at the parent
    /// node, on the affected side, must be updated, along with any
    /// affected heights up to the root. If the process results in an
    /// imbalance of more than one between the left height and right
    /// height of a node in the ancestor chain, the corresponding
    /// subtree must be rebalanced.
    ///
    /// Parent-child edge updates are handled in `insert_tree_node()`
    /// and `remove_tree_node()`, while the height retracing process is
    /// handled here.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_id` : Node ID of affected tree node.
    /// * `operation`: `INCREMENT` or `DECREMENT`, the change in height
    ///   on the affected side.
    /// * `side`: `LEFT` or `RIGHT`, the side on which the height is
    ///   affected.
    ///
    /// # Testing
    ///
    /// Tests are designed to evaluate both true and false outcomes for
    /// all logical branches, with each relevant test covering multiple
    /// conditional branches, optionally via a retrace back to the root.
    ///
    /// See `test_rotate_right_1()` and `test_rotate_left_2()` for more
    /// information on their corresponding reference diagrams.
    ///
    /// `if (height_left != height_right)`
    ///
    /// | Exercises `true`       | Exercises `false`              |
    /// |------------------------|--------------------------------|
    /// | `test_rotate_left_2()` | `test_retrace_insert_remove()` |
    ///
    /// `if (height_left > height_right)`
    ///
    /// | Exercises `true`        | Exercises `false`      |
    /// |-------------------------|------------------------|
    /// | `test_rotate_right_1()` | `test_rotate_left_2()` |
    ///
    /// `if (imbalance > 1)`
    ///
    /// | Exercises `true`       | Exercises `false`              |
    /// |------------------------|--------------------------------|
    /// | `test_rotate_left_2()` | `test_retrace_insert_remove()` |
    ///
    /// `if (left_heavy)`
    ///
    /// | Exercises `true`        | Exercises `false`      |
    /// |-------------------------|------------------------|
    /// | `test_rotate_right_1()` | `test_rotate_left_2()` |
    ///
    /// `if (parent == (NIL as u64))`
    ///
    /// | Exercises `true`        | Exercises `false`      |
    /// |-------------------------|------------------------|
    /// | `test_rotate_right_1()` | `test_rotate_left_2()` |
    ///
    /// `if (new_subtree_root != (NIL as u64))`
    ///
    /// | Exercises `true`        | Exercises `false`              |
    /// |-------------------------|--------------------------------|
    /// | `test_rotate_right_1()` | `test_retrace_insert_remove()` |
    ///
    /// `if (delta == 0)`
    ///
    /// | Exercises `true`       | Exercises `false`              |
    /// |------------------------|--------------------------------|
    /// | `test_rotate_left_2()` | `test_retrace_insert_remove()` |
    ///
    /// Assorted tests indexed at `remove_tree_node_follow_up()`
    /// additionally exercise retracing logic.
    ///
    /// ## Reference diagram
    ///
    /// For `test_retrace_insert_remove()`, insert node d and retrace
    /// from node c, then remove node d and retrace from c again.
    ///
    /// Pre-insertion:
    ///
    /// >       4
    /// >      / \
    /// >     3   5
    ///
    /// Pre-removal:
    ///
    /// >       node b -> 4
    /// >                / \
    /// >     node a -> 3   5 <- node c
    /// >                    \
    /// >                     6 <- node d
    ///
    /// Post-removal:
    ///
    /// >       4
    /// >      / \
    /// >     3   5
    fun retrace<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_id: u64,
        operation: bool,
        side: bool
    ) {
        let delta = 1; // Mark height change of one for first iteration.
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // Mutably borrow node under consideration.
        let node_ref_mut =
            table_with_length::borrow_mut(nodes_ref_mut, node_id);
        loop {
            // Get parent field of node under review.
            let parent = (((node_ref_mut.bits >> SHIFT_PARENT) &
                (HI_NODE_ID as u128)) as u64);
            let (height_left, height_right, height, height_old) =
                retrace_update_heights(node_ref_mut, side, operation, delta);
            // Flag no rebalancing via null new subtree root.
            let new_subtree_root = (NIL as u64);
            if (height_left != height_right) {
                // If node not balanced:
                // Determine if node is left-heavy, and calculate the
                // imbalance of the node (the difference in height
                // between node's two subtrees).
                let (left_heavy, imbalance) = if (height_left > height_right)
                    (true, height_left - height_right) else
                    (false, height_right - height_left);
                if (imbalance > 1) {
                    // If imbalance greater than 1:
                    // Get shift amount for child on heavy side.
                    let child_shift = if (left_heavy) SHIFT_CHILD_LEFT else
                        SHIFT_CHILD_RIGHT;
                    // Get child ID from node bits.
                    let child_id = (((node_ref_mut.bits >> child_shift) &
                        (HI_NODE_ID as u128)) as u64);
                    // Rebalance, storing node ID of new subtree root
                    // and new subtree height.
                    (new_subtree_root, height) = retrace_rebalance(
                        avlq_ref_mut, node_id, child_id, left_heavy);
                };
            }; // Corresponding subtree has been optionally rebalanced.
            if (parent == (NIL as u64)) {
                // If just retraced root:
                // If just rebalanced at root:
                if (new_subtree_root != (NIL as u64)) {
                    // Reassign bits for root MSBs:
                    avlq_ref_mut.bits = avlq_ref_mut.bits &
                        // Clear out field via mask unset at field bits.
                        (HI_128 ^ ((HI_NODE_ID as u128) >> BITS_PER_BYTE)) |
                        // Mask in new bits.
                        ((new_subtree_root as u128) >> BITS_PER_BYTE);
                    avlq_ref_mut.root_lsbs = // Set AVL queue root LSBs.
                        ((new_subtree_root & HI_BYTE) as u8);
                }; // AVL queue root now current for actual root.
                return // Stop looping.
            } else {
                // If just retraced node not at root:
                // Prepare to optionally iterate again.
                (node_ref_mut, operation, side, delta) =
                    retrace_prep_iterate(avlq_ref_mut, parent, node_id,
                        new_subtree_root, height, height_old);
                // Return if current iteration did not result in height
                // change for corresponding subtree.
                if (delta == 0) return;
                // Store parent ID as node ID for next iteration.
                node_id = parent;
            };
        }
    }

    /// Prepare for an optional next retrace iteration.
    ///
    /// Inner function for `retrace()`, should only be called if just
    /// retraced below the root of the AVL queue.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `parent_id`: Node ID of next ancestor in retrace chain.
    /// * `node_id`: Node ID at root of subtree just retraced, before
    ///   any optional rebalancing took place.
    /// * `new_subtree_root`: Node ID of new subtree root for when
    ///   rebalancing took place, `NIL` if no rebalancing.
    /// * `height`: Height of subtree after retrace.
    /// * `height_old`: Height of subtree before retrace.
    ///
    /// # Returns
    ///
    /// * `&mut TreeNode`: Mutable reference to next ancestor.
    /// * `bool`: `INCREMENT` or `DECREMENT`, the change in height for
    ///   the subtree just retraced. Evaluates to `DECREMENT` when
    ///   height does not change.
    /// * `bool`: `LEFT` or `RIGHT`, the side on which the retraced
    ///   subtree was a child to the next ancestor.
    /// * `u8`: Change in height of subtree due to retrace, evaluates to
    ///   0 when height does not change.
    ///
    /// # Testing
    ///
    /// * `test_retrace_prep_iterate_1()`
    /// * `test_retrace_prep_iterate_2()`
    /// * `test_retrace_prep_iterate_3()`
    ///
    /// ## Case 1
    ///
    /// * Side is `LEFT`.
    /// * Subtree rebalanced.
    /// * Operation is `DECREMENT`.
    /// * Actual change in height.
    ///
    /// ## Case 2
    ///
    /// * Side is `RIGHT`.
    /// * Subtree rebalanced.
    /// * Operation is `DECREMENT`.
    /// * No change in height.
    ///
    /// ## Case 3
    ///
    /// * Side is `RIGHT`.
    /// * Subtree not rebalanced.
    /// * Operation is `INCREMENT`.
    fun retrace_prep_iterate<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        parent_id: u64,
        node_id: u64,
        new_subtree_root: u64,
        height: u8,
        height_old: u8,
    ): (
        &mut TreeNode,
        bool,
        bool,
        u8
    ) {
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // Mutably borrow parent to subtree just retraced.
        let node_ref_mut =
            table_with_length::borrow_mut(nodes_ref_mut, parent_id);
        // Get parent's left child.
        let left_child = ((node_ref_mut.bits >> SHIFT_CHILD_LEFT) &
            (HI_NODE_ID as u128) as u64);
        // Flag side on which retracing operation took place.
        let side = if (left_child == node_id) LEFT else RIGHT;
        // If subtree rebalanced:
        if (new_subtree_root != (NIL as u64)) {
            // Get corresponding child field shift amount.
            let child_shift = if (side == LEFT)
                SHIFT_CHILD_LEFT else SHIFT_CHILD_RIGHT;
            // Reassign bits for new child field.
            node_ref_mut.bits = node_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << child_shift)) |
                // Mask in new bits.
                ((new_subtree_root as u128) << child_shift)
        }; // Parent-child edge updated.
        // Determine retrace operation type and height delta.
        let (operation, delta) = if (height > height_old)
            (INCREMENT, height - height_old) else
            (DECREMENT, height_old - height);
        // Return mutable reference to parent node, operation performed,
        // side of operation, and corresponding change in height.
        (node_ref_mut, operation, side, delta)
    }

    /// Rebalance a subtree, returning new root and height.
    ///
    /// Inner function for `retrace()`.
    ///
    /// Updates state for nodes in subtree, but not for potential parent
    /// to subtree.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_id_x`: Node ID of subtree root.
    /// * `node_id_z`: Node ID of child to subtree root, on subtree
    ///   root's heavy side.
    /// * `node_x_left_heavy`: `true` if node x is left-heavy.
    ///
    /// # Returns
    ///
    /// * `u64`: Tree node ID of new subtree root after rotation.
    /// * `u8`: Height of subtree after rotation.
    ///
    /// # Node x status
    ///
    /// Node x can be either left-heavy or right heavy. In either case,
    /// consider that node z has left child and right child fields.
    ///
    /// ## Node x left-heavy
    ///
    /// >             n_x
    /// >            /
    /// >          n_z
    /// >         /   \
    /// >     z_c_l   z_c_r
    ///
    /// ## Node x right-heavy
    ///
    /// >       n_x
    /// >          \
    /// >          n_z
    /// >         /   \
    /// >     z_c_l   z_c_r
    ///
    /// # Testing
    ///
    /// * `test_rotate_left_1()`
    /// * `test_rotate_left_2()`
    /// * `test_rotate_left_right_1()`
    /// * `test_rotate_left_right_2()`
    /// * `test_rotate_right_1()`
    /// * `test_rotate_right_2()`
    /// * `test_rotate_right_left_1()`
    /// * `test_rotate_right_left_2()`
    fun retrace_rebalance<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_z_id: u64,
        node_x_left_heavy: bool,
    ): (
        u64,
        u8
    ) {
        let node_z_ref = // Immutably borrow node z.
            table_with_length::borrow(&avlq_ref_mut.tree_nodes, node_z_id);
        let bits = node_z_ref.bits; // Get node z bits.
        // Get node z's left height, right height, and child fields.
        let (node_z_height_left, node_z_height_right,
            node_z_child_left, node_z_child_right) =
            ((((bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_CHILD_LEFT) & (HI_NODE_ID as u128)) as u64),
                (((bits >> SHIFT_CHILD_RIGHT) & (HI_NODE_ID as u128)) as u64));
        // Return result of rotation. If node x is left-heavy:
        return (if (node_x_left_heavy)
            // If node z is right-heavy, rotate left-right
            (if (node_z_height_right > node_z_height_left)
                retrace_rebalance_rotate_left_right(
                    avlq_ref_mut, node_x_id, node_z_id, node_z_child_right,
                    node_z_height_left)
                // Otherwise node z is not right-heavy so rotate right.
            else retrace_rebalance_rotate_right(
                avlq_ref_mut, node_x_id, node_z_id, node_z_child_right,
                node_z_height_right))
        else // If node x is right-heavy:
        // If node z is left-heavy, rotate right-left
            (if (node_z_height_left > node_z_height_right)
                retrace_rebalance_rotate_right_left(
                    avlq_ref_mut, node_x_id, node_z_id, node_z_child_left,
                    node_z_height_right)
                // Otherwise node z is not left-heavy so rotate left.
            else retrace_rebalance_rotate_left(
                avlq_ref_mut, node_x_id, node_z_id, node_z_child_left,
                node_z_height_left)))
    }

    /// Rotate left during rebalance.
    ///
    /// Inner function for `retrace_rebalance()`.
    ///
    /// Updates state for nodes in subtree, but not for potential parent
    /// to subtree.
    ///
    /// Here, subtree root node x is right-heavy, with right child
    /// node z that is not left-heavy. Node x has an optional tree 1
    /// as its left child subtree, and node z has optional trees 2 and
    /// 3 as its left and right child subtrees, respectively.
    ///
    /// Pre-rotation:
    ///
    /// >        n_x
    /// >       /   \
    /// >     t_1   n_z
    /// >          /   \
    /// >        t_2   t_3
    ///
    /// Post-rotation:
    ///
    /// >           n_z
    /// >          /   \
    /// >        n_x   t_3
    /// >       /   \
    /// >     t_1   t_2
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of subtree root pre-rotation.
    /// * `node_z_id`: Node ID of subtree root post-rotation.
    /// * `tree_2_id`: Node z's left child field.
    /// * `node_z_height_left`: Node z's left height.
    ///
    /// # Returns
    ///
    /// * `u64`: Node z's ID.
    /// * `u8`: The height of the subtree rooted at node z,
    ///   post-rotation.
    ///
    /// # Reference rotations
    ///
    /// ## Case 1
    ///
    /// * Tree 2 null.
    /// * Node x left height greater than or equal to right height
    ///   post-rotation.
    /// * Node z right height greater than or equal to left height
    ///   post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >     4 <- node x
    /// >      \
    /// >       6 <- node z
    /// >        \
    /// >         8 <- tree 3
    ///
    /// Post-rotation:
    ///
    /// >                 6 <- node z
    /// >                / \
    /// >     node x -> 4   8 <- tree 3
    ///
    /// ## Case 2
    ///
    /// * Tree 2 not null.
    /// * Node x left height not greater than or equal to right height
    ///   post-rotation.
    /// * Node z right height not greater than or equal to left height
    ///   post-rotation.
    /// * Remove node d, then retrace from node x.
    ///
    /// Pre-removal:
    ///
    /// >                   3 <- node a
    /// >                  / \
    /// >       node b -> 2   5
    /// >                /   / \
    /// >     node c -> 1   4   7
    /// >            node d ^  / \
    /// >                     6   8
    ///
    /// Pre-rotation:
    ///
    /// >             3
    /// >            / \
    /// >           2   5 <- node x
    /// >          /     \
    /// >         1       7 <- node z
    /// >                / \
    /// >     tree 2 -> 6   8 <- tree 3
    ///
    /// Post-rotation:
    ///
    /// >         3
    /// >        / \
    /// >       2   7 <- node z
    /// >      /   / \
    /// >     1   5   8 <- tree 3
    /// >          \
    /// >           6 <- tree 2
    ///
    /// # Testing
    ///
    /// * `test_rotate_left_1()`
    /// * `test_rotate_left_2()`
    fun retrace_rebalance_rotate_left<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_z_id: u64,
        tree_2_id: u64,
        node_z_height_left: u8
    ): (
        u64,
        u8
    ) {
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        if (tree_2_id != (NIL as u64)) {
            // If tree 2 is not empty:
            let tree_2_ref_mut = // Mutably borrow tree 2 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_2_id);
            // Reassign bits for new parent field:
            tree_2_ref_mut.bits = tree_2_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_x_id as u128) << SHIFT_PARENT);
        };
        let node_x_ref_mut = // Mutably borrow node x.
            table_with_length::borrow_mut(nodes_ref_mut, node_x_id);
        let node_x_height_left = (((node_x_ref_mut.bits >> SHIFT_HEIGHT_LEFT) &
            (HI_HEIGHT as u128)) as u8); // Get node x left height.
        // Node x's right height is from transferred tree 2.
        let node_x_height_right = node_z_height_left;
        let node_x_parent = (((node_x_ref_mut.bits >> SHIFT_PARENT) &
            (HI_NODE_ID as u128)) as u64); // Get node x parent field.
        // Reassign bits for right child, right height, and parent:
        node_x_ref_mut.bits = node_x_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_2_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_x_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_z_id as u128) << SHIFT_PARENT);
        // Determine height of tree rooted at x.
        let node_x_height = if (node_x_height_left >= node_x_height_right)
            node_x_height_left else node_x_height_right;
        // Get node z left height.
        let node_z_height_left = node_x_height + 1;
        let node_z_ref_mut = // Mutably borrow node z.
            table_with_length::borrow_mut(nodes_ref_mut, node_z_id);
        // Reassign bits for left child, left height, and parent:
        node_z_ref_mut.bits = node_z_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((node_x_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_z_height_left as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_x_parent as u128) << SHIFT_PARENT);
        let node_z_height_right = (((node_z_ref_mut.bits >> SHIFT_HEIGHT_RIGHT)
            & (HI_HEIGHT as u128)) as u8); // Get node z right height.
        // Determine height of tree rooted at z.
        let node_z_height = if (node_z_height_right >= node_z_height_left)
            node_z_height_right else node_z_height_left;
        (node_z_id, node_z_height) // Return new subtree root, height.
    }

    /// Rotate left-right during rebalance.
    ///
    /// Inner function for `retrace_rebalance()`.
    ///
    /// Updates state for nodes in subtree, but not for potential parent
    /// to subtree.
    ///
    /// Here, subtree root node x is left-heavy, with left child node
    /// z that is right-heavy. Node z has as its right child node y.
    ///
    /// Node z has an optional tree 1 as its left child subtree, node
    /// y has optional trees 2 and 3 as its left and right child
    /// subtrees, respectively, and node x has an optional tree 4 as its
    /// right child subtree.
    ///
    /// Double rotations result in a subtree root with a balance factor
    /// of zero, such that node y is has the same left and right height
    /// post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >           n_x
    /// >          /   \
    /// >        n_z   t_4
    /// >       /   \
    /// >     t_1   n_y
    /// >          /   \
    /// >        t_2   t_3
    ///
    /// Post-rotation:
    ///
    /// >              n_y
    /// >          ___/   \___
    /// >        n_z         n_x
    /// >       /   \       /   \
    /// >     t_1   t_2   t_3   t_4
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of subtree root pre-rotation.
    /// * `node_z_id`: Node ID of subtree left child pre-rotation.
    /// * `node_y_id`: Node ID of subtree root post-rotation.
    /// * `node_z_height_left`: Node z's left height pre-rotation.
    ///
    /// # Procedure
    ///
    /// * Inspect node y's fields.
    /// * Optionally update tree 2's parent field.
    /// * Optionally update tree 3's parent field.
    /// * Update node x's left child and parent fields.
    /// * Update node z's right child and parent fields.
    /// * Update node y's children and parent fields.
    ///
    /// # Reference rotations
    ///
    /// ## Case 1
    ///
    /// * Tree 2 null.
    /// * Tree 3 not null.
    /// * Node z right height not greater than or equal to left height
    ///   post-rotation.
    /// * Remove node r, then retrace from node x.
    ///
    /// Pre-removal:
    ///
    /// >         5
    /// >        / \
    /// >       2   6 <- node r
    /// >      / \   \
    /// >     1   3   7
    /// >          \
    /// >           4
    ///
    /// Pre-rotation:
    ///
    /// >                   5 <- node x
    /// >                  / \
    /// >       node z -> 2   7 <- tree 4
    /// >                / \
    /// >     tree 1 -> 1   3 <- node y
    /// >                    \
    /// >                     4 <- tree 3
    ///
    /// Post-rotation:
    ///
    /// >                   3 <- node y
    /// >                  / \
    /// >       node z -> 2   5 <- node x
    /// >                /   / \
    /// >     tree 1 -> 1   4   7 <- tree 4
    /// >                   ^ tree 3
    ///
    /// ## Case 2
    ///
    /// * Tree 2 not null.
    /// * Tree 3 null.
    /// * Node z right height greater than or equal to left height
    ///   post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >                   8 <- node x
    /// >                  / \
    /// >       node z -> 2   9 <- tree 4
    /// >                / \
    /// >     tree 1 -> 1   6 <- node y
    /// >                  /
    /// >       tree 2 -> 5
    ///
    /// Post-rotation:
    ///
    /// >                   6 <- node y
    /// >                  / \
    /// >       node z -> 2   8 <- node x
    /// >                / \   \
    /// >     tree 1 -> 1   5   9 <- tree 4
    /// >                   ^ tree 2
    ///
    /// # Testing
    ///
    /// * `test_rotate_left_right_1()`
    /// * `test_rotate_left_right_2()`
    fun retrace_rebalance_rotate_left_right<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_z_id: u64,
        node_y_id: u64,
        node_z_height_left: u8
    ): (
        u64,
        u8
    ) {
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // Immutably borrow node y.
        let node_y_ref = table_with_length::borrow(nodes_ref_mut, node_y_id);
        let y_bits = node_y_ref.bits; // Get node y bits.
        // Get node y's left and right height, and tree 2 and 3 IDs.
        let (node_y_height_left, node_y_height_right, tree_2_id, tree_3_id) =
            ((((y_bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((y_bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8),
                (((y_bits >> SHIFT_CHILD_LEFT) & (HI_NODE_ID as u128)) as u64),
                (((y_bits >> SHIFT_CHILD_RIGHT) & (HI_NODE_ID as u128)) as u64));
        if (tree_2_id != (NIL as u64)) {
            // If tree 2 not null:
            let tree_2_ref_mut = // Mutably borrow tree 2 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_2_id);
            // Reassign bits for new parent field:
            tree_2_ref_mut.bits = tree_2_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_z_id as u128) << SHIFT_PARENT);
        };
        if (tree_3_id != (NIL as u64)) {
            // If tree 3 not null:
            let tree_3_ref_mut = // Mutably borrow tree 3 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_3_id);
            // Reassign bits for new parent field:
            tree_3_ref_mut.bits = tree_3_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_x_id as u128) << SHIFT_PARENT);
        };
        let node_x_ref_mut = // Mutably borrow node x.
            table_with_length::borrow_mut(nodes_ref_mut, node_x_id);
        // Node x's left height is from transferred tree 3.
        let node_x_height_left = node_y_height_right;
        let node_x_parent = (((node_x_ref_mut.bits >> SHIFT_PARENT) &
            (HI_NODE_ID as u128)) as u64); // Store node x parent field.
        // Reassign bits for left child, left height, and parent:
        node_x_ref_mut.bits = node_x_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_3_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_x_height_left as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_y_id as u128) << SHIFT_PARENT);
        let node_z_ref_mut = // Mutably borrow node z.
            table_with_length::borrow_mut(nodes_ref_mut, node_z_id);
        // Node z's right height is from transferred tree 2.
        let node_z_height_right = node_y_height_left;
        // Reassign bits for right child, right height, and parent:
        node_z_ref_mut.bits = node_z_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_2_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_z_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_y_id as u128) << SHIFT_PARENT);
        // Determine height of tree rooted at z.
        let node_z_height = if (node_z_height_right >= node_z_height_left)
            node_z_height_right else node_z_height_left;
        // Get node y's post-rotation height (same on left and right).
        let node_y_height = node_z_height + 1;
        let node_y_ref_mut = // Mutably borrow node y.
            table_with_length::borrow_mut(nodes_ref_mut, node_y_id);
        // Reassign bits for both child edges, and parent.
        node_y_ref_mut.bits = node_y_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((node_z_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_x_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_y_height as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_y_height as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_x_parent as u128) << SHIFT_PARENT);
        (node_y_id, node_y_height) // Return new subtree root, height.
    }

    /// Rotate right during rebalance.
    ///
    /// Inner function for `retrace_rebalance()`.
    ///
    /// Updates state for nodes in subtree, but not for potential parent
    /// to subtree.
    ///
    /// Here, subtree root node x is left-heavy, with left child
    /// node z that is not right-heavy. Node x has an optional tree 3
    /// as its right child subtree, and node z has optional trees 1 and
    /// 2 as its left and right child subtrees, respectively.
    ///
    /// Pre-rotation:
    ///
    /// >           n_x
    /// >          /   \
    /// >        n_z   t_3
    /// >       /   \
    /// >     t_1   t_2
    ///
    /// Post-rotation:
    ///
    /// >        n_z
    /// >       /   \
    /// >     t_1   n_x
    /// >          /   \
    /// >        t_2   t_3
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of subtree root pre-rotation.
    /// * `node_z_id`: Node ID of subtree root post-rotation.
    /// * `tree_2_id`: Node z's right child field.
    /// * `node_z_height_right`: Node z's right height.
    ///
    /// # Returns
    ///
    /// * `u64`: Node z's ID.
    /// * `u8`: The height of the subtree rooted at node z,
    ///   post-rotation.
    ///
    /// # Reference rotations
    ///
    /// ## Case 1
    ///
    /// * Tree 2 null.
    /// * Node x right height greater than or equal to left height
    ///   post-rotation.
    /// * Node z left height greater than or equal to right height
    ///   post-rotation.
    /// * Simulates inserting tree 1, then retracing from node z.
    ///
    /// Pre-insertion:
    ///
    /// >       8
    /// >      /
    /// >     6
    ///
    /// Pre-rotation:
    ///
    /// >         8 <- node x
    /// >        /
    /// >       6 <- node z
    /// >      /
    /// >     4 <- tree 1
    ///
    /// Post-rotation:
    ///
    /// >                 6 <- node z
    /// >                / \
    /// >     tree 1 -> 4   8 <- node x
    ///
    /// ## Case 2
    ///
    /// * Tree 2 not null.
    /// * Node x right height not greater than or equal to left height
    ///   post-rotation.
    /// * Node z left height not greater than or equal to right height
    ///   post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >                   7 <- node x
    /// >                  /
    /// >                 4 <- node z
    /// >                / \
    /// >     tree 1 -> 3   5 <- tree 2
    ///
    /// Post-rotation:
    ///
    /// >                 4 <- node z
    /// >                / \
    /// >     tree 1 -> 3   7 <- node x
    /// >                  /
    /// >                 5 <- tree 2
    ///
    /// # Testing
    ///
    /// * `test_rotate_right_1()`
    /// * `test_rotate_right_2()`
    fun retrace_rebalance_rotate_right<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_z_id: u64,
        tree_2_id: u64,
        node_z_height_right: u8
    ): (
        u64,
        u8
    ) {
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        if (tree_2_id != (NIL as u64)) {
            // If tree 2 is not empty:
            let tree_2_ref_mut = // Mutably borrow tree 2 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_2_id);
            // Reassign bits for new parent field:
            tree_2_ref_mut.bits = tree_2_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_x_id as u128) << SHIFT_PARENT);
        };
        let node_x_ref_mut = // Mutably borrow node x.
            table_with_length::borrow_mut(nodes_ref_mut, node_x_id);
        let node_x_height_right = (((node_x_ref_mut.bits >> SHIFT_HEIGHT_RIGHT)
            & (HI_HEIGHT as u128)) as u8); // Get node x right height.
        // Node x's left height is from transferred tree 2.
        let node_x_height_left = node_z_height_right;
        let node_x_parent = (((node_x_ref_mut.bits >> SHIFT_PARENT) &
            (HI_NODE_ID as u128)) as u64); // Get node x parent field.
        // Reassign bits for left child, left height, and parent:
        node_x_ref_mut.bits = node_x_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_2_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_x_height_left as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_z_id as u128) << SHIFT_PARENT);
        // Determine height of tree rooted at x.
        let node_x_height = if (node_x_height_right >= node_x_height_left)
            node_x_height_right else node_x_height_left;
        // Get node z right height.
        let node_z_height_right = node_x_height + 1;
        let node_z_ref_mut = // Mutably borrow node z.
            table_with_length::borrow_mut(nodes_ref_mut, node_z_id);
        // Reassign bits for right child, right height, and parent:
        node_z_ref_mut.bits = node_z_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((node_x_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_z_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_x_parent as u128) << SHIFT_PARENT);
        let node_z_height_left = (((node_z_ref_mut.bits >> SHIFT_HEIGHT_LEFT) &
            (HI_HEIGHT as u128)) as u8); // Get node z left height.
        // Determine height of tree rooted at z.
        let node_z_height = if (node_z_height_left >= node_z_height_right)
            node_z_height_left else node_z_height_right;
        (node_z_id, node_z_height) // Return new subtree root, height.
    }

    /// Rotate right-left during rebalance.
    ///
    /// Inner function for `retrace_rebalance()`.
    ///
    /// Updates state for nodes in subtree, but not for potential parent
    /// to subtree.
    ///
    /// Here, subtree root node x is right-heavy, with right child node
    /// z that is left-heavy. Node z has as its left child node y.
    ///
    /// Node x has an optional tree 1 as its left child subtree, node
    /// y has optional trees 2 and 3 as its left and right child
    /// subtrees, respectively, and node z has an optional tree 4 as its
    /// right child subtree.
    ///
    /// Double rotations result in a subtree root with a balance factor
    /// of zero, such that node y is has the same left and right height
    /// post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >        n_x
    /// >       /   \
    /// >     t_1   n_z
    /// >          /   \
    /// >        n_y   t_4
    /// >       /   \
    /// >     t_2   t_3
    ///
    /// Post-rotation:
    ///
    /// >              n_y
    /// >          ___/   \___
    /// >        n_x         n_z
    /// >       /   \       /   \
    /// >     t_1   t_2   t_3   t_4
    ///
    /// # Parameters
    ///
    /// * `avlq_ref_mut`: Mutable reference to AVL queue.
    /// * `node_x_id`: Node ID of subtree root pre-rotation.
    /// * `node_z_id`: Node ID of subtree right child pre-rotation.
    /// * `node_y_id`: Node ID of subtree root post-rotation.
    /// * `node_z_height_right`: Node z's right height pre-rotation.
    ///
    /// # Procedure
    ///
    /// * Inspect node y's fields.
    /// * Optionally update tree 2's parent field.
    /// * Optionally update tree 3's parent field.
    /// * Update node x's right child and parent fields.
    /// * Update node z's left child and parent fields.
    /// * Update node y's children and parent fields.
    ///
    /// # Reference rotations
    ///
    /// ## Case 1
    ///
    /// * Tree 2 not null.
    /// * Tree 3 null.
    /// * Node z left height not greater than or equal to right height
    ///   post-rotation.
    ///
    /// Pre-rotation:
    ///
    /// >                 2 <- node x
    /// >                / \
    /// >     tree 1 -> 1   8 <- node z
    /// >                  / \
    /// >       node y -> 4   9 <- tree 4
    /// >                /
    /// >               3 <- tree 2
    ///
    /// Post-rotation:
    ///
    /// >                   4 <- node y
    /// >                  / \
    /// >       node x -> 2   8 <- node z
    /// >                / \   \
    /// >     tree 1 -> 1   3   9 <- tree 4
    /// >                   ^ tree 2
    ///
    /// ## Case 2
    ///
    /// * Tree 2 null.
    /// * Tree 3 not null.
    /// * Node z left height greater than or equal to right height
    ///   post-rotation.
    /// * Remove node r, then retrace from node x.
    ///
    /// Pre-removal:
    ///
    /// >                 3
    /// >                / \
    /// >     node r -> 2   6
    /// >              /   / \
    /// >             1   4   7
    /// >                  \
    /// >                   5
    ///
    /// Pre-rotation:
    ///
    /// >                 3 <- node x
    /// >                / \
    /// >     tree 1 -> 1   6 <- node z
    /// >                  / \
    /// >       node y -> 4   7 <- tree 4
    /// >                  \
    /// >                   5 <- tree 3
    ///
    /// Post-rotation:
    ///
    /// >                   4 <- node y
    /// >                  / \
    /// >       node x -> 3   6 <- node z
    /// >                /   / \
    /// >     tree 1 -> 1   5   7 <- tree 4
    /// >                   ^ tree 3
    ///
    /// # Testing
    ///
    /// * `test_rotate_right_left_1()`
    /// * `test_rotate_right_left_2()`
    fun retrace_rebalance_rotate_right_left<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_x_id: u64,
        node_z_id: u64,
        node_y_id: u64,
        node_z_height_right: u8
    ): (
        u64,
        u8
    ) {
        // Mutably borrow tree nodes table.
        let nodes_ref_mut = &mut avlq_ref_mut.tree_nodes;
        // Immutably borrow node y.
        let node_y_ref = table_with_length::borrow(nodes_ref_mut, node_y_id);
        let y_bits = node_y_ref.bits; // Get node y bits.
        // Get node y's left and right height, and tree 2 and 3 IDs.
        let (node_y_height_left, node_y_height_right, tree_2_id, tree_3_id) =
            ((((y_bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((y_bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8),
                (((y_bits >> SHIFT_CHILD_LEFT) & (HI_NODE_ID as u128)) as u64),
                (((y_bits >> SHIFT_CHILD_RIGHT) & (HI_NODE_ID as u128)) as u64));
        if (tree_2_id != (NIL as u64)) {
            // If tree 2 not null:
            let tree_2_ref_mut = // Mutably borrow tree 2 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_2_id);
            // Reassign bits for new parent field:
            tree_2_ref_mut.bits = tree_2_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_x_id as u128) << SHIFT_PARENT);
        };
        if (tree_3_id != (NIL as u64)) {
            // If tree 3 not null:
            let tree_3_ref_mut = // Mutably borrow tree 3 root.
                table_with_length::borrow_mut(nodes_ref_mut, tree_3_id);
            // Reassign bits for new parent field:
            tree_3_ref_mut.bits = tree_3_ref_mut.bits &
                // Clear out field via mask unset at field bits.
                (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_PARENT)) |
                // Mask in new bits.
                ((node_z_id as u128) << SHIFT_PARENT);
        };
        let node_x_ref_mut = // Mutably borrow node x.
            table_with_length::borrow_mut(nodes_ref_mut, node_x_id);
        // Node x's right height is from transferred tree 2.
        let node_x_height_right = node_y_height_left;
        let node_x_parent = (((node_x_ref_mut.bits >> SHIFT_PARENT) &
            (HI_NODE_ID as u128)) as u64); // Store node x parent field.
        // Reassign bits for right child, right height, and parent:
        node_x_ref_mut.bits = node_x_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_2_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_x_height_right as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_y_id as u128) << SHIFT_PARENT);
        let node_z_ref_mut = // Mutably borrow node z.
            table_with_length::borrow_mut(nodes_ref_mut, node_z_id);
        // Node z's left height is from transferred tree 3.
        let node_z_height_left = node_y_height_right;
        // Reassign bits for left child, left height, and parent:
        node_z_ref_mut.bits = node_z_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((tree_3_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_z_height_left as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_y_id as u128) << SHIFT_PARENT);
        // Determine height of tree rooted at z.
        let node_z_height = if (node_z_height_left >= node_z_height_right)
            node_z_height_left else node_z_height_right;
        // Get node y's post-rotation height (same on left and right).
        let node_y_height = node_z_height + 1;
        let node_y_ref_mut = // Mutably borrow node y.
            table_with_length::borrow_mut(nodes_ref_mut, node_y_id);
        // Reassign bits for both child edges, and parent.
        node_y_ref_mut.bits = node_y_ref_mut.bits &
            // Clear out fields via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_CHILD_LEFT) |
                ((HI_NODE_ID as u128) << SHIFT_CHILD_RIGHT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_LEFT) |
                ((HI_HEIGHT as u128) << SHIFT_HEIGHT_RIGHT) |
                ((HI_NODE_ID as u128) << SHIFT_PARENT))) |
            // Mask in new bits.
            ((node_x_id as u128) << SHIFT_CHILD_LEFT) |
            ((node_z_id as u128) << SHIFT_CHILD_RIGHT) |
            ((node_y_height as u128) << SHIFT_HEIGHT_LEFT) |
            ((node_y_height as u128) << SHIFT_HEIGHT_RIGHT) |
            ((node_x_parent as u128) << SHIFT_PARENT);
        (node_y_id, node_y_height) // Return new subtree root, height.
    }

    /// Update height fields during retracing.
    ///
    /// Inner function for `retrace()`.
    ///
    /// # Parameters
    ///
    /// * `node_ref_mut`: Mutable reference to a node that needs to have
    ///   its height fields updated during retrace.
    /// * `side`: `LEFT` or `RIGHT`, the side on which the node's height
    ///   needs to be updated.
    /// * `operation`: `INCREMENT` or `DECREMENT`, the kind of change in
    ///   the height field for the given side.
    /// * `delta`: The amount of height change for the operation.
    ///
    /// # Returns
    ///
    /// * `u8`: The left height of the node after updating height.
    /// * `u8`: The right height of the node after updating height.
    /// * `u8`: The height of the node before updating height.
    /// * `u8`: The height of the node after updating height.
    ///
    /// # Testing
    ///
    /// * `test_retrace_update_heights_1()`
    /// * `test_retrace_update_heights_2()`
    ///
    /// ## Case 1
    ///
    /// * Left height is greater than or equal to right height
    ///   pre-retrace.
    /// * Side is `LEFT`.
    /// * Operation is `DECREMENT`.
    /// * Left height is greater than or equal to right height
    ///   post-retrace.
    ///
    /// ## Case 2
    ///
    /// * Left height is not greater than or equal to right height
    ///   pre-retrace.
    /// * Side is `RIGHT`.
    /// * Operation is `INCREMENT`.
    /// * Left height is not greater than or equal to right height
    ///   post-retrace.
    fun retrace_update_heights(
        node_ref_mut: &mut TreeNode,
        side: bool,
        operation: bool,
        delta: u8
    ): (
        u8,
        u8,
        u8,
        u8
    ) {
        let bits = node_ref_mut.bits; // Get node's field bits.
        // Get node's left height, right height, and parent fields.
        let (height_left, height_right) =
            ((((bits >> SHIFT_HEIGHT_LEFT) & (HI_HEIGHT as u128)) as u8),
                (((bits >> SHIFT_HEIGHT_RIGHT) & (HI_HEIGHT as u128)) as u8));
        let height_old = if (height_left >= height_right) height_left else
            height_right; // Get height of node before retracing.
        // Get height field and shift amount for operation side.
        let (height_field, height_shift) = if (side == LEFT)
            (height_left, SHIFT_HEIGHT_LEFT) else
            (height_right, SHIFT_HEIGHT_RIGHT);
        // Get updated height field for side.
        let height_field = if (operation == INCREMENT) height_field + delta
        else height_field - delta;
        // Reassign bits for corresponding height field:
        node_ref_mut.bits = bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_HEIGHT as u128) << height_shift)) |
            // Mask in new bits.
            ((height_field as u128) << height_shift);
        // Reassign local height to that of indicated field.
        if (side == LEFT) height_left = height_field else
            height_right = height_field;
        let height = if (height_left >= height_right) height_left else
            height_right; // Get height of node after update.
        (height_left, height_right, height, height_old)
    }

    /// Search in AVL queue for closest match to seed key.
    ///
    /// Return immediately if empty tree, otherwise get node ID of root
    /// node. Then start walking down nodes, branching left whenever the
    /// seed key is less than a node's key, right whenever the seed
    /// key is greater than a node's key, and returning when the seed
    /// key equals a node's key. Also return if there is no child to
    /// branch to on a given side.
    ///
    /// The "match" node is the node last walked before returning.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref`: Immutable reference to AVL queue.
    /// * `seed_key`: Seed key to search for.
    ///
    /// # Returns
    ///
    /// * `u64`: Node ID of match node, or `NIL` if empty tree.
    /// * `Option<bool>`: None if empty tree or if match key equals seed
    ///   key, `LEFT` if seed key is less than match key but match node
    ///   has no left child, `RIGHT` if seed key is greater than match
    ///   key but match node has no right child.
    ///
    /// # Assumptions
    ///
    /// * Seed key fits in 32 bits.
    ///
    /// # Reference diagram
    ///
    /// >               4 <- ID 1
    /// >              / \
    /// >     ID 5 -> 2   8 <- ID 2
    /// >                / \
    /// >       ID 4 -> 6   10 <- ID 3
    ///
    /// | Seed key | Match key | Node ID | Side  |
    /// |----------|-----------|---------|-------|
    /// | 2        | 2         | 5       | None  |
    /// | 7        | 6         | 4       | Right |
    /// | 9        | 10        | 3       | Left  |
    /// | 4        | 4         | 1       | None  |
    ///
    /// # Testing
    ///
    /// * `test_search()`
    public fun search<V>(
        avlq_ref: &AVLqueue<V>,
        seed_key: u64
    ): (
        u64,
        Option<bool>
    ) {
        let root_msbs = // Get root MSBs.
            ((avlq_ref.bits & ((HI_NODE_ID as u128) >> BITS_PER_BYTE)) as u64);
        let node_id = // Shift over, mask in LSBs, store as search node.
            (root_msbs << BITS_PER_BYTE) | (avlq_ref.root_lsbs as u64);
        // If no node at root, return as such, with empty option.
        if (node_id == (NIL as u64)) return (node_id, option::none());
        // Mutably borrow tree nodes table.
        let nodes_ref = &avlq_ref.tree_nodes;
        loop {
            // Begin walking down tree nodes:
            let node_ref = // Mutably borrow node having given ID.
                table_with_length::borrow(nodes_ref, node_id);
            // Get insertion key encoded in search node's bits.
            let node_key = (((node_ref.bits >> SHIFT_INSERTION_KEY) &
                (HI_INSERTION_KEY as u128)) as u64);
            // If search key equals seed key, return node's ID and
            // empty option.
            if (seed_key == node_key) return (node_id, option::none());
            // Get bitshift for child node ID and side based on
            // inequality comparison between seed key and node key.
            let (child_shift, child_side) = if (seed_key < node_key)
                (SHIFT_CHILD_LEFT, LEFT) else (SHIFT_CHILD_RIGHT, RIGHT);
            let child_id = (((node_ref.bits >> child_shift) &
                (HI_NODE_ID as u128)) as u64); // Get child node ID.
            // If no child on given side, return match node's ID
            // and option with given side.
            if (child_id == (NIL as u64)) return
                (node_id, option::some(child_side));
            // Otherwise continue walk at given child.
            node_id = child_id;
        }
    }

    /// Traverse from tree node to inorder predecessor or successor.
    ///
    /// # Parameters
    ///
    /// * `avlq_ref`: Immutable reference to AVL queue.
    /// * `start_node_id`: Tree node ID of node to traverse from.
    /// * `target`: Either `PREDECESSOR` or `SUCCESSOR`.
    ///
    /// # Conventions
    ///
    /// Traversal starts at the "start node" and ends at the "target
    /// node", if any.
    ///
    /// # Returns
    ///
    /// * `u64`: Insertion key of target node, or `NIL`.
    /// * `u64`: List node ID for head of doubly linked list in
    ///   target node, or `NIL`.
    /// * `u64`: List node ID for tail of doubly linked list in
    ///   target node, or `NIL`.
    ///
    /// # Membership considerations
    ///
    /// * Aborts if no tree node in AVL queue with given start node ID.
    /// * Returns all `NIL` if start node is sole node at root.
    /// * Returns all `NIL` if no predecessor or successor.
    /// * Returns all `NIL` if start node ID indicates inactive node.
    ///
    /// # Predecessor
    ///
    /// 1. If start node has left child, return maximum node in left
    ///    child's right subtree.
    /// 2. Otherwise, walk upwards until reaching a node that had last
    ///    walked node as the root of its right subtree.
    ///
    /// # Successor
    ///
    /// 1. If start node has right child, return minimum node in right
    ///    child's left subtree.
    /// 2. Otherwise, walk upwards until reaching a node that had last
    ///    walked node as the root of its left subtree.
    ///
    /// # Reference diagram
    ///
    /// >                 5
    /// >            ____/ \____
    /// >           2           8
    /// >          / \         / \
    /// >         1   3       7   9
    /// >              \     /
    /// >               4   6
    ///
    /// Inserted in following sequence:
    ///
    /// | Insertion key | Sequence number |
    /// |---------------|-----------------|
    /// | 5             | 1               |
    /// | 8             | 2               |
    /// | 2             | 3               |
    /// | 1             | 4               |
    /// | 3             | 5               |
    /// | 7             | 6               |
    /// | 9             | 7               |
    /// | 4             | 8               |
    /// | 6             | 9               |
    ///
    /// # Testing
    ///
    /// * `test_traverse()`
    fun traverse<V>(
        avlq_ref: &AVLqueue<V>,
        start_node_id: u64,
        target: bool
    ): (
        u64,
        u64,
        u64
    ) {
        // Immutably borrow tree nodes table.
        let nodes_ref = &avlq_ref.tree_nodes;
        // Immutably borrow start node.
        let node_ref = table_with_length::borrow(nodes_ref, start_node_id);
        // Determine child and subtree side based on target.
        let (child_shift, subtree_shift) = if (target == PREDECESSOR)
            (SHIFT_CHILD_LEFT, SHIFT_CHILD_RIGHT) else
            (SHIFT_CHILD_RIGHT, SHIFT_CHILD_LEFT);
        let bits = node_ref.bits; // Get node bits.
        // Get node ID of relevant child to start node.
        let child = (((bits >> child_shift) & (HI_NODE_ID as u128)) as u64);
        if (child == (NIL as u64)) {
            // If no such child:
            child = start_node_id; // Set child as start node.
            loop {
                // Start upward walk.
                let parent = // Get parent field from node bits.
                    (((bits >> SHIFT_PARENT) & (HI_NODE_ID as u128)) as u64);
                // Return all null if no parent.
                if (parent == (NIL as u64)) return
                    ((NIL as u64), (NIL as u64), (NIL as u64));
                // Otherwise, immutably borrow parent node.
                node_ref = table_with_length::borrow(nodes_ref, parent);
                bits = node_ref.bits; // Get node bits.
                let subtree = // Get subtree field for break side.
                    (((bits >> subtree_shift) & (HI_NODE_ID as u128)) as u64);
                // If child from indicated subtree, break out of loop.
                if (subtree == child) break;
                // Otherwise store node ID for next iteration.
                child = parent;
            };
        } else {
            // If start node has child on relevant side:
            loop {
                // Start downward walk.
                // Immutably borrow child node.
                node_ref = table_with_length::borrow(nodes_ref, child);
                bits = node_ref.bits; // Get node bits.
                child = // Get node ID of child in relevant subtree.
                    (((bits >> subtree_shift) & (HI_NODE_ID as u128)) as u64);
                // If no subtree left to check, break out of loop.
                if (child == (NIL as u64)) break; // Else iterate again.
            }
        };
        let bits = node_ref.bits; // Get node bits.
        // Return insertion key, list head, and list tail.
        ((((bits >> SHIFT_INSERTION_KEY) & (HI_INSERTION_KEY as u128)) as u64),
            (((bits >> SHIFT_LIST_HEAD) & (HI_NODE_ID as u128)) as u64),
            (((bits >> SHIFT_LIST_TAIL) & (HI_NODE_ID as u128)) as u64))
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// When a char in a bytestring is neither 0 nor 1.
    const E_BIT_NOT_0_OR_1: u64 = 100;

    // Test-only error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Immutably borrow list node having given node ID.
    fun borrow_list_node_test<V>(
        avlq_ref: &AVLqueue<V>,
        node_id: u64
    ): &ListNode {
        table_with_length::borrow(&avlq_ref.list_nodes, node_id)
    }

    #[test_only]
    /// Immutably borrow tree node having given node ID.
    fun borrow_tree_node_test<V>(
        avlq_ref: &AVLqueue<V>,
        node_id: u64
    ): &TreeNode {
        table_with_length::borrow(&avlq_ref.tree_nodes, node_id)
    }

    #[test_only]
    /// Immutably borrow value option having given node ID.
    public fun borrow_value_option_test<V>(
        avlq_ref: &AVLqueue<V>,
        node_id: u64
    ): &Option<V> {
        table::borrow(&avlq_ref.values, node_id)
    }

    #[test_only]
    /// Drop AVL queue.
    fun drop_avlq_test<V>(
        avlq: AVLqueue<V>
    ) {
        // Unpack all fields, dropping those that are not tables.
        let AVLqueue { bits: _, root_lsbs: _, tree_nodes, list_nodes, values } =
            avlq;
        // Drop all tables.
        table_with_length::drop_unchecked(tree_nodes);
        table_with_length::drop_unchecked(list_nodes);
        table::drop_unchecked(values);
    }

    #[test_only]
    /// Flip access key sort order bit.
    public fun flip_access_key_sort_order_bit_test(
        access_key: u64
    ): u64 {
        access_key ^ ((BIT_FLAG_ASCENDING as u64) << (SHIFT_ACCESS_SORT_ORDER))
    }

    #[test_only]
    /// Get list node ID encoded in an access key.
    ///
    /// # Testing
    ///
    /// * `test_access_key_getters()`
    public fun get_access_key_list_node_id_test(
        access_key: u64
    ): u64 {
        (access_key >> SHIFT_ACCESS_LIST_NODE_ID) & HI_NODE_ID
    }

    #[test_only]
    /// Get tree node ID encoded in an access key.
    ///
    /// # Testing
    ///
    /// * `test_access_key_getters()`
    fun get_access_key_tree_node_id_test(
        access_key: u64
    ): u64 {
        (access_key >> SHIFT_ACCESS_TREE_NODE_ID) & HI_NODE_ID
    }

    #[test_only]
    /// Like `get_child_left_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_child_left_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_child_left_test(tree_node_ref) // Return left child field.
    }

    #[test_only]
    /// Return left child node ID indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_child_left_test()`
    fun get_child_left_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_CHILD_LEFT) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Return right child node ID indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_child_right_test()`
    fun get_child_right_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_CHILD_RIGHT)) &
            (HI_NODE_ID as u128) as u64)
    }

    #[test_only]
    /// Like `get_child_right_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_child_right_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_child_right_test(tree_node_ref) // Return right child field.
    }

    #[test_only]
    /// Return head insertion key indicated by given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun get_head_key_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_HEAD_KEY) &
            (HI_INSERTION_KEY as u128)) as u64)
    }

    #[test_only]
    /// Return head list node ID indicated by given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun get_head_node_id_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_HEAD_NODE_ID) & (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Like `get_height_left_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_height_left_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u8 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_height_left_test(tree_node_ref) // Return left height.
    }

    #[test_only]
    /// Return left height indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_height_left_test()`
    fun get_height_left_test(
        tree_node_ref: &TreeNode
    ): u8 {
        (((tree_node_ref.bits >> SHIFT_HEIGHT_LEFT) &
            (HI_HEIGHT as u128)) as u8)
    }

    #[test_only]
    /// Return right height indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_height_right_test()`
    fun get_height_right_test(
        tree_node_ref: &TreeNode
    ): u8 {
        (((tree_node_ref.bits >> SHIFT_HEIGHT_RIGHT) &
            (HI_HEIGHT as u128)) as u8)
    }

    #[test_only]
    /// Like `get_height_right_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_height_right_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u8 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_height_right_test(tree_node_ref) // Return right height.
    }

    #[test_only]
    /// Like `get_insertion_key_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_insertion_key_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_insertion_key_test(tree_node_ref) // Return insertion key.
    }

    #[test_only]
    /// Return insertion key indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_insertion_key_test()`
    fun get_insertion_key_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_INSERTION_KEY) &
            (HI_INSERTION_KEY as u128)) as u64)
    }

    #[test_only]
    /// Like `get_list_head_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_list_head_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_list_head_test(tree_node_ref) // Return list head.
    }

    #[test_only]
    /// Return list head node ID indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_list_head_test()`
    fun get_list_head_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_LIST_HEAD) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Return node ID of last node and if last node is a tree node,
    /// for given list node.
    ///
    /// # Testing
    ///
    /// * `test_get_list_last_test()`
    fun get_list_last_test(
        list_node_ref: &ListNode
    ): (
        u64,
        bool
    ) {
        // Get virtual last field.
        let last_field = ((list_node_ref.last_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.last_lsbs as u64);
        let tree_node_flag = (((last_field >> SHIFT_NODE_TYPE) &
            (BIT_FLAG_TREE_NODE as u64)) as u8); // Get tree node flag.
        // Return node ID, and if last node is a tree node.
        ((last_field & HI_NODE_ID), tree_node_flag == BIT_FLAG_TREE_NODE)
    }

    #[test_only]
    /// Like `get_list_last_test()`, but accepts list node ID inside
    /// given AVL queue.
    fun get_list_last_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): (
        u64,
        bool
    ) {
        let list_node_ref = // Immutably borrow list node.
            table_with_length::borrow(&avlq_ref.list_nodes, list_node_id);
        get_list_last_test(list_node_ref) // Return last field data.
    }

    #[test_only]
    /// Return only node ID from `get_list_last_by_id_test()`.
    fun get_list_last_node_id_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): u64 {
        // Get last node ID.
        let (node_id, _) = get_list_last_by_id_test(avlq_ref, list_node_id);
        node_id // Return it.
    }

    #[test_only]
    /// Return node ID of next node and if next node is a tree node,
    /// for given list node.
    ///
    /// # Testing
    ///
    /// * `test_get_list_next_test()`
    fun get_list_next_test(
        list_node_ref: &ListNode
    ): (
        u64,
        bool
    ) {
        // Get virtual next field.
        let next_field = ((list_node_ref.next_msbs as u64) << BITS_PER_BYTE) |
            (list_node_ref.next_lsbs as u64);
        let tree_node_flag = (((next_field >> SHIFT_NODE_TYPE) &
            (BIT_FLAG_TREE_NODE as u64)) as u8); // Get tree node flag.
        // Return node ID, and if next node is a tree node.
        ((next_field & HI_NODE_ID), tree_node_flag == BIT_FLAG_TREE_NODE)
    }

    #[test_only]
    /// Like `get_list_next_test()`, but accepts list node ID inside
    /// given AVL queue.
    fun get_list_next_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): (
        u64,
        bool
    ) {
        let list_node_ref = // Immutably borrow list node.
            table_with_length::borrow(&avlq_ref.list_nodes, list_node_id);
        get_list_next_test(list_node_ref) // Return next field data.
    }

    #[test_only]
    /// Return only node ID from `get_list_next_by_id_test()`.
    fun get_list_next_node_id_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): u64 {
        // Get next node ID.
        let (node_id, _) = get_list_next_by_id_test(avlq_ref, list_node_id);
        node_id // Return it.
    }

    #[test_only]
    /// Like `get_list_tail_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_list_tail_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_list_tail_test(tree_node_ref) // Return list tail.
    }

    #[test_only]
    /// Return list tail node ID indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_list_tail_test()`
    fun get_list_tail_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_LIST_TAIL) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Return node ID at top of inactive list node stack indicated by
    /// given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_get_list_top_test()`
    fun get_list_top_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_LIST_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Like `get_parent_test()`, but accepts tree node ID inside given
    /// AVL queue.
    fun get_parent_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_parent_test(tree_node_ref) // Return parent field.
    }

    #[test_only]
    /// Return parent node ID indicated by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_parent_test()`
    fun get_parent_test(
        tree_node_ref: &TreeNode
    ): u64 {
        (((tree_node_ref.bits >> SHIFT_PARENT) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Return tail insertion key indicated by given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun get_tail_key_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_TAIL_KEY) &
            (HI_INSERTION_KEY as u128)) as u64)
    }

    #[test_only]
    /// Return tail list node ID indicated by given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun get_tail_node_id_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_TAIL_NODE_ID) & (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Like `get_tree_next_test()`, but accepts tree node ID inside
    /// given AVL queue.
    fun get_tree_next_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        tree_node_id: u64
    ): u64 {
        let tree_node_ref = // Immutably borrow tree node.
            table_with_length::borrow(&avlq_ref.tree_nodes, tree_node_id);
        get_tree_next_test(tree_node_ref) // Return parent field.
    }

    #[test_only]
    /// Return node ID of next inactive tree node in stack, indicated
    /// by given tree node.
    ///
    /// # Testing
    ///
    /// * `test_get_tree_next_test()`
    fun get_tree_next_test(
        tree_node_ref: &TreeNode
    ): u64 {
        ((tree_node_ref.bits & (HI_64 as u128)) as u64) & HI_NODE_ID
    }

    #[test_only]
    /// Return node ID at top of inactive tree node stack indicated by
    /// given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_get_tree_top_test()`
    fun get_tree_top_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        (((avlq_ref.bits >> SHIFT_TREE_STACK_TOP) &
            (HI_NODE_ID as u128)) as u64)
    }

    #[test_only]
    /// Return root node ID indicated by AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_root_test()`
    fun get_root_test<V>(
        avlq_ref: &AVLqueue<V>
    ): u64 {
        // Get MSBs.
        let msbs = avlq_ref.bits & ((HI_NODE_ID as u128) >> BITS_PER_BYTE);
        // Mask in LSBs and return.
        ((msbs << BITS_PER_BYTE) as u64) | (avlq_ref.root_lsbs as u64)
    }

    #[test_only]
    /// Return copy of value for given node ID.
    fun get_value_test<V: copy>(
        avlq_ref: &AVLqueue<V>,
        node_id: u64
    ): V {
        // Borrow value option.
        let value_option_ref = borrow_value_option_test(avlq_ref, node_id);
        // Return copy of value.
        *option::borrow(value_option_ref)
    }

    #[test_only]
    /// Return only is tree node flag from `get_list_last_by_id_test()`.
    fun is_tree_node_list_last_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): bool {
        let (_, is_tree_node) = // Check if last node is tree node.
            get_list_last_by_id_test(avlq_ref, list_node_id);
        is_tree_node // Return flag.
    }

    #[test_only]
    /// Return only is tree node flag from `get_list_next_by_id_test()`.
    fun is_tree_node_list_next_by_id_test<V>(
        avlq_ref: &AVLqueue<V>,
        list_node_id: u64
    ): bool {
        let (_, is_tree_node) = // Check if next node is tree node.
            get_list_next_by_id_test(avlq_ref, list_node_id);
        is_tree_node // Return flag.
    }

    #[test_only]
    /// Set head insertion key in given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun set_head_key_test<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64
    ) {
        // Reassign bits:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_INSERTION_KEY as u128) << SHIFT_HEAD_KEY as u128)) |
            // Mask in new bits.
            ((key as u128) << SHIFT_HEAD_KEY)
    }

    #[test_only]
    /// Set head list node ID in given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun set_head_node_id_test<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_id: u64
    ) {
        // Reassign bits:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID as u128) << SHIFT_HEAD_NODE_ID as u128)) |
            // Mask in new bits.
            ((node_id as u128) << SHIFT_HEAD_NODE_ID)
    }

    #[test_only]
    /// Set root node ID.
    ///
    /// # Testing
    ///
    /// * `test_set_get_root_test()`
    fun set_root_test<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        root_node_id: u64
    ) {
        // Reassign bits for root MSBs:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ ((HI_NODE_ID >> BITS_PER_BYTE) as u128)) |
            // Mask in new bits.
            ((root_node_id as u128) >> BITS_PER_BYTE);
        // Set root LSBs.
        avlq_ref_mut.root_lsbs = ((root_node_id & HI_BYTE) as u8);
    }

    #[test_only]
    /// Set tail insertion key in given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun set_tail_key_test<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        key: u64
    ) {
        // Reassign bits:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ (((HI_INSERTION_KEY as u128) << SHIFT_TAIL_KEY) as u128))
            // Mask in new bits.
            | ((key as u128) << SHIFT_TAIL_KEY)
    }

    #[test_only]
    /// Set tail list node ID in given AVL queue.
    ///
    /// # Testing
    ///
    /// * `test_set_get_head_tail_test()`
    fun set_tail_node_id_test<V>(
        avlq_ref_mut: &mut AVLqueue<V>,
        node_id: u64
    ) {
        // Reassign bits:
        avlq_ref_mut.bits = avlq_ref_mut.bits &
            // Clear out field via mask unset at field bits.
            (HI_128 ^ (((HI_NODE_ID as u128) << SHIFT_TAIL_NODE_ID) as u128)) |
            // Mask in new bits.
            ((node_id as u128) << SHIFT_TAIL_NODE_ID)
    }

    #[test_only]
    /// Return a `u128` corresponding to provided byte string `s`. The
    /// byte should only contain only "0"s and "1"s, up to 128
    /// characters max (e.g. `b"100101...10101010"`).
    ///
    /// # Testing
    ///
    /// * `test_u_128_64()`
    /// * `test_u_128_failure()`
    public fun u_128(
        s: vector<u8>
    ): u128 {
        let n = vector::length<u8>(&s); // Get number of bits.
        let r = 0; // Initialize result to 0.
        let i = 0; // Start loop at least significant bit.
        while (i < n) {
            // While there are bits left to review.
            // Get bit under review.
            let b = *vector::borrow<u8>(&s, n - 1 - i);
            if (b == 0x31) {
                // If the bit is 1 (0x31 in ASCII):
                // OR result with the correspondingly leftshifted bit.
                r = r | (1 << (i as u8));
                // Otherwise, assert bit is marked 0 (0x30 in ASCII).
            } else assert!(b == 0x30, E_BIT_NOT_0_OR_1);
            i = i + 1; // Proceed to next-least-significant bit.
        };
        r // Return result.
    }

    #[test_only]
    /// Return `u128` corresponding to concatenated result of `a`, `b`,
    /// `c`, and `d`. Useful for line-wrapping long byte strings, and
    /// inspection via 32-bit sections.
    ///
    /// # Testing
    ///
    /// * `test_u_128_64()`
    public fun u_128_by_32(
        a: vector<u8>,
        b: vector<u8>,
        c: vector<u8>,
        d: vector<u8>,
    ): u128 {
        vector::append<u8>(&mut c, d); // Append d onto c.
        vector::append<u8>(&mut b, c); // Append c onto b.
        vector::append<u8>(&mut a, b); // Append b onto a.
        u_128(a) // Return u128 equivalent of concatenated bytestring.
    }

    #[test_only]
    /// Wrapper for `u_128()`, casting return to `u64`.
    ///
    /// # Testing
    ///
    /// * `test_u_128_64()`
    public fun u_64(s: vector<u8>): u64 { (u_128(s) as u64) }

    #[test_only]
    /// Wrapper for `u_128_by_32()`, accepting only two inputs, with
    /// casted return to `u64`.
    public fun u_64_by_32(
        a: vector<u8>,
        b: vector<u8>
    ): u64 {
        // Get u128 for given inputs, cast to u64.
        (u_128_by_32(a, b, b"", b"") as u64)
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    /// Verify successful returns.
    fun test_access_key_getters() {
        // Assert access key not marked ascending if no bits set.
        assert!(!is_ascending_access_key((NIL as u64)), 0);
        // Declare encoded information for access key.
        let tree_node_id = u_64(b"10000000000001");
        let list_node_id = u_64(b"11000000000011");
        let insertion_key = u_64(b"10000000000000000000000000000001");
        let access_key = u_64_by_32(
            b"00010000000000001110000000000111",
            //   ^ bits 47-60 ^^ bits 33-46 ^^ bit 32
            b"10000000000000000000000000000001");
        // Assert access key getter returns.
        assert!(get_access_key_tree_node_id_test(access_key)
            == tree_node_id, 0);
        assert!(get_access_key_list_node_id_test(access_key)
            == list_node_id, 0);
        assert!(is_ascending_access_key(access_key), 0);
        assert!(get_access_key_insertion_key(access_key)
            == insertion_key, 0);
    }

    #[test]
    /// Verify successful state updates.
    fun test_borrow_borrow_mut() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        let access_key_0 = insert(&mut avlq, 0, 123); // Insert key.
        // Assert borrow.
        assert!(*borrow(&avlq, access_key_0) == 123, 0);
        *borrow_mut(&mut avlq, access_key_0) = 456; // Mutate directly.
        assert!(*borrow_head(&avlq) == 456, 0); // Assert head borrow.
        *borrow_head_mut(&mut avlq) = 789; // Mutate via head lookup.
        assert!(*borrow_tail(&avlq) == 789, 0); // Assert tail borrow.
        *borrow_tail_mut(&mut avlq) = 321; // Mutate via tail lookup.
        insert(&mut avlq, 1, 654); // Insert new tail.
        assert!(*borrow_head(&avlq) == 321, 0); // Assert head borrow.
        assert!(*borrow_tail(&avlq) == 654, 0); // Assert tail borrow.
        *borrow_tail_mut(&mut avlq) = 987; // Mutate via tail lookup.
        assert!(*borrow_tail(&avlq) == 987, 0); // Assert tail borrow.
        *borrow_head_mut(&mut avlq) = 123; // Mutate via head lookup.
        // Assert borrow.
        assert!(*borrow(&avlq, access_key_0) == 123, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_child_left_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111111111111111111111100000",
                //                          ^ bit 69
                b"00000001111111111111111111111111",
                //       ^ bit 56
                b"11111111111111111111111111111111")
        };
        // Assert left child node ID.
        assert!(get_child_left_test(&tree_node) == u_64(b"10000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_child_right_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111100000000000011111111111",
                //        ^ bit 55     ^ bit 42
                b"11111111111111111111111111111111")
        };
        assert!(// Assert right child node ID.
            get_child_right_test(&tree_node) == u_64(b"10000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful returns.
    fun test_get_head_tail_key() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Assert head and tail keys indicated as none.
        assert!(option::is_none(&get_head_key(&avlq)), 0);
        assert!(option::is_none(&get_tail_key(&avlq)), 0);
        // Insert minimum insertion key.
        let access_key_lo = insert(&mut avlq, 0, 0);
        // Assert head and tail keys indicate the same.
        assert!(*option::borrow(&get_head_key(&avlq)) == 0, 0);
        assert!(*option::borrow(&get_tail_key(&avlq)) == 0, 0);
        // Insert maximum insertion key.
        let access_key_hi = insert(&mut avlq, HI_INSERTION_KEY, HI_64);
        // Assert head and tail keys indicate differently.
        assert!(*option::borrow(&get_head_key(&avlq)) == 0, 0);
        assert!(*option::borrow(&get_tail_key(&avlq)) == HI_INSERTION_KEY, 0);
        // Remove minimum insertion key.
        remove(&mut avlq, access_key_lo);
        // Assert head and tail keys indicate the same.
        assert!(*option::borrow(&get_head_key(&avlq)) == HI_INSERTION_KEY, 0);
        assert!(*option::borrow(&get_tail_key(&avlq)) == HI_INSERTION_KEY, 0);
        // Remove maximum insertion key.
        remove(&mut avlq, access_key_hi);
        // Assert head and tail keys indicated as none.
        assert!(option::is_none(&get_head_key(&avlq)), 0);
        assert!(option::is_none(&get_tail_key(&avlq)), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful returns for `get_height()` reference diagram.
    fun test_get_height() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Assert height indicated as none.
        assert!(option::is_none(&get_height(&avlq)), 0);
        insert(&mut avlq, 4, 0); // Insert key 4.
        // Assert height indicated as 0.
        assert!(*option::borrow(&get_height(&avlq)) == 0, 0);
        insert(&mut avlq, 5, 0); // Insert key 5.
        // Assert height indicated as 1.
        assert!(*option::borrow(&get_height(&avlq)) == 1, 0);
        insert(&mut avlq, 3, 0); // Insert key 3.
        // Assert height still indicated as 1.
        assert!(*option::borrow(&get_height(&avlq)) == 1, 0);
        insert(&mut avlq, 1, 0); // Insert key 1.
        // Assert height indicated as 2.
        assert!(*option::borrow(&get_height(&avlq)) == 2, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_height_left_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11100011111111111111111111111111",
                //  ^   ^ bits 89-93
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111")
        };
        // Assert left height.
        assert!(get_height_left_test(&tree_node) == (u_64(b"10001") as u8), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_height_right_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111000111111111111111111111",
                //       ^   ^ bits 84-88
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111")
        };
        assert!(// Assert right height.
            get_height_right_test(&tree_node) == (u_64(b"10001") as u8), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_insertion_key_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11100000000000000000000000000000",
                //  ^ bit 125
                b"01111111111111111111111111111111",
                // ^ bit 94
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111")
        };
        // Assert insertion key
        assert!(get_insertion_key_test(&tree_node) ==
            u_64(b"10000000000000000000000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_list_head_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111111111000000000",
                //                      ^ bit 41
                b"00011111111111111111111111111111")
        };
        //   ^ bit 28
        // Assert list head node ID.
        assert!(get_list_head_test(&tree_node) == u_64(b"10000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_list_last_test() {
        // Declare list node.
        let list_node = ListNode {
            last_msbs: (u_64(b"00100000") as u8),
            last_lsbs: (u_64(b"00000001") as u8),
            next_msbs: 0,
            next_lsbs: 0
        };
        // Get last node info.
        let (node_id, is_tree_node) = get_list_last_test(&list_node);
        // Assert last node ID.
        assert!(node_id == u_64(b"10000000000001"), 0);
        // Assert not marked as tree node.
        assert!(!is_tree_node, 0);
        // Flag as tree node.
        list_node.last_msbs = (u_64(b"01100000") as u8);
        // Get last node info.
        (node_id, is_tree_node) = get_list_last_test(&list_node);
        // Assert last node ID unchanged.
        assert!(node_id == u_64(b"10000000000001"), 0);
        // Assert marked as tree node.
        assert!(is_tree_node, 0);
        ListNode { last_msbs: _, last_lsbs: _, next_msbs: _, next_lsbs: _ } =
            list_node; // Unpack list node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_list_next_test() {
        // Declare list node.
        let list_node = ListNode {
            last_msbs: 0,
            last_lsbs: 0,
            next_msbs: (u_64(b"00100000") as u8),
            next_lsbs: (u_64(b"00000001") as u8)
        };
        // Get next node info.
        let (node_id, is_tree_node) = get_list_next_test(&list_node);
        // Assert next node ID.
        assert!(node_id == u_64(b"10000000000001"), 0);
        // Assert not marked as tree node.
        assert!(!is_tree_node, 0);
        // Flag as tree node.
        list_node.next_msbs = (u_64(b"01100000") as u8);
        // Get next node info.
        (node_id, is_tree_node) = get_list_next_test(&list_node);
        // Assert next node ID unchanged.
        assert!(node_id == u_64(b"10000000000001"), 0);
        // Assert marked as tree node.
        assert!(is_tree_node, 0);
        ListNode { last_msbs: _, last_lsbs: _, next_msbs: _, next_lsbs: _ } =
            list_node; // Unpack list node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_list_tail_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111000000000000111111111111111")
        };
        //    ^ bit 27     ^ bit 14
        // Assert list tail node ID.
        assert!(get_list_tail_test(&tree_node) == u_64(b"10000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_list_top_test() {
        let avlq = AVLqueue<u8> {
            // Create empty AVL queue.
            bits: u_128_by_32(
                b"11111111111111111000000000000111",
                //                ^ bit 111    ^ bit 98
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111"),
            root_lsbs: (NIL as u8),
            tree_nodes: table_with_length::new(),
            list_nodes: table_with_length::new(),
            values: table::new(),
        };
        // Assert list top.
        assert!(get_list_top_test(&avlq) == u_64(b"10000000000001"), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_parent_test() {
        let tree_node = TreeNode {
            bits: u_128_by_32(// Create tree node.
                b"11111111111111111111111111111111",
                b"11111111111110000000000001111111",
                //            ^ bit 83     ^ bit 70
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111")
        };
        // Assert parent node ID.
        assert!(get_parent_test(&tree_node) == u_64(b"10000000000001"), 0);
        let TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_tree_next_test() {
        // Declare tree node.
        let tree_node = TreeNode {
            bits: u_128_by_32(
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111110000000000001")
        };
        assert!(// Assert next node ID.
            get_tree_next_test(&tree_node) == u_64(b"10000000000001"), 0);
        TreeNode { bits: _ } = tree_node; // Unpack tree node.
    }

    #[test]
    /// Verify successful extraction.
    fun test_get_tree_top_test() {
        let avlq = AVLqueue<u8> {
            // Create empty AVL queue.
            bits: u_128_by_32(
                b"11100000000000011111111111111111",
                //  ^ bit 125    ^ bit 112
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111",
                b"11111111111111111111111111111111"),
            root_lsbs: (NIL as u8),
            tree_nodes: table_with_length::new(),
            list_nodes: table_with_length::new(),
            values: table::new(),
        };
        // Assert tree top.
        assert!(get_tree_top_test(&avlq) == u_64(b"10000000000001"), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify expected returns.
    fun test_contains_active_list_node_id() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        let access_key = HI_64; // Declare bogus access key.
        // Assert marked as not having active list node ID encoded.
        assert!(!contains_active_list_node_id(&avlq, access_key), 0);
        // Insert arbitrary key, reassigning access key.
        access_key = insert(&mut avlq, 1, 0);
        // Assert marked as having active list node ID encoded.
        assert!(contains_active_list_node_id(&avlq, access_key), 0);
        remove(&mut avlq, access_key); // Remove key-value pair.
        // Assert marked as not having active list node ID encoded.
        assert!(!contains_active_list_node_id(&avlq, access_key), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify expected returns.
    fun test_has_key() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Assert not marked as having arbitrary key.
        assert!(!has_key(&avlq, 1), 0);
        insert(&mut avlq, 1, 0); // Insert arbitrary key.
        // Assert marked as having key.
        assert!(has_key(&avlq, 1), 0);
        // Assert not marked as having different key.
        assert!(!has_key(&avlq, 2), 0);
        assert!(!has_key(&avlq, HI_INSERTION_KEY), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_INSERTION_KEY_TOO_LARGE)]
    /// Verify failure for insertion key too big.
    fun test_has_key_too_big() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Attempt invalid invocation.
        has_key(&avlq, HI_INSERTION_KEY + 1);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify insertion sequence from `insert()`.
    fun test_insert() {
        // Init ascending AVL queue with allocated nodes.
        let avlq = new(ASCENDING, 7, 3);
        // Insert per reference diagram, storing access keys and testing
        // local tail check updates.
        let access_key_3_9 = insert(&mut avlq, 3, 9);
        assert!(is_local_tail(&avlq, access_key_3_9), 0);
        let access_key_4_8 = insert(&mut avlq, 4, 8);
        assert!(is_local_tail(&avlq, access_key_4_8), 0);
        let access_key_5_7 = insert(&mut avlq, 5, 7);
        assert!(is_local_tail(&avlq, access_key_5_7), 0);
        let access_key_3_6 = insert(&mut avlq, 3, 6);
        assert!(is_local_tail(&avlq, access_key_3_6), 0);
        assert!(!is_local_tail(&avlq, access_key_3_9), 0);
        let access_key_5_5 = insert(&mut avlq, 5, 5);
        assert!(!is_local_tail(&avlq, access_key_5_7), 0);
        assert!(is_local_tail(&avlq, access_key_5_5), 0);
        // Declare expected node IDs per initial node allocations.
        let tree_node_id_3_9 = 7;
        let tree_node_id_4_8 = 6;
        let tree_node_id_5_7 = 5;
        let tree_node_id_3_6 = 7;
        let tree_node_id_5_5 = 5;
        let list_node_id_3_9 = 3;
        let list_node_id_4_8 = 2;
        let list_node_id_5_7 = 1;
        let list_node_id_3_6 = 4;
        let list_node_id_5_5 = 5;
        let tree_node_id_3 = tree_node_id_3_9;
        let tree_node_id_4 = tree_node_id_4_8;
        let tree_node_id_5 = tree_node_id_5_7;
        // Assert access key insertion keys.
        assert!(get_access_key_insertion_key(access_key_3_9) == 3, 0);
        assert!(get_access_key_insertion_key(access_key_4_8) == 4, 0);
        assert!(get_access_key_insertion_key(access_key_5_7) == 5, 0);
        assert!(get_access_key_insertion_key(access_key_3_6) == 3, 0);
        assert!(get_access_key_insertion_key(access_key_5_5) == 5, 0);
        // Assert access key sort order.
        assert!(is_ascending_access_key(access_key_3_9), 0);
        assert!(is_ascending_access_key(access_key_4_8), 0);
        assert!(is_ascending_access_key(access_key_5_7), 0);
        assert!(is_ascending_access_key(access_key_3_6), 0);
        assert!(is_ascending_access_key(access_key_5_5), 0);
        // Assert access key tree node IDs.
        assert!(get_access_key_tree_node_id_test(access_key_3_9)
            == tree_node_id_3_9, 0);
        assert!(get_access_key_tree_node_id_test(access_key_4_8)
            == tree_node_id_4_8, 0);
        assert!(get_access_key_tree_node_id_test(access_key_5_7)
            == tree_node_id_5_7, 0);
        assert!(get_access_key_tree_node_id_test(access_key_3_6)
            == tree_node_id_3_6, 0);
        assert!(get_access_key_tree_node_id_test(access_key_5_5)
            == tree_node_id_5_5, 0);
        // Assert access key list node IDs.
        assert!(get_access_key_list_node_id_test(access_key_3_9)
            == list_node_id_3_9, 0);
        assert!(get_access_key_list_node_id_test(access_key_4_8)
            == list_node_id_4_8, 0);
        assert!(get_access_key_list_node_id_test(access_key_5_7)
            == list_node_id_5_7, 0);
        assert!(get_access_key_list_node_id_test(access_key_3_6)
            == list_node_id_3_6, 0);
        assert!(get_access_key_list_node_id_test(access_key_5_5)
            == list_node_id_5_5, 0);
        // Assert root tree node ID.
        assert!(get_root_test(&avlq) == tree_node_id_4, 0);
        // Assert inactive tree node stack top
        assert!(get_tree_top_test(&avlq) == 4, 0);
        // Assert empty inactive list node stack.
        assert!(get_list_top_test(&avlq) == (NIL as u64), 0);
        // Assert AVL queue head and tail.
        assert!(get_head_node_id_test(&avlq) == list_node_id_3_9, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_5_5, 0);
        assert!(get_head_key_test(&avlq) == 3, 0);
        assert!(get_tail_key_test(&avlq) == 5, 0);
        // Assert all tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id_3) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_node_id_3) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_node_id_3) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id_3)
            == tree_node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_node_id_3)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_node_id_3)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id_3)
            == list_node_id_3_9, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id_3)
            == list_node_id_3_6, 0);
        assert!(get_tree_next_by_id_test(&avlq, tree_node_id_3)
            == (NIL as u64), 0);
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id_4) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_node_id_4) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_node_id_4) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id_4)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, tree_node_id_4)
            == tree_node_id_3, 0);
        assert!(get_child_right_by_id_test(&avlq, tree_node_id_4)
            == tree_node_id_5, 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id_4)
            == list_node_id_4_8, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id_4)
            == list_node_id_4_8, 0);
        assert!(get_tree_next_by_id_test(&avlq, tree_node_id_4)
            == (NIL as u64), 0);
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id_5) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_node_id_5) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_node_id_5) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id_5)
            == tree_node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_node_id_5)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_node_id_5)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id_5)
            == list_node_id_5_7, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id_5)
            == list_node_id_5_5, 0);
        assert!(get_tree_next_by_id_test(&avlq, tree_node_id_5)
            == (NIL as u64), 0);
        // Assert all list node state.
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id_3_9)
            == tree_node_id_3, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_node_id_3_9),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id_3_9)
            == list_node_id_3_6, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_node_id_3_9),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id_3_6)
            == list_node_id_3_9, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_node_id_3_6),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id_3_6)
            == tree_node_id_3, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_node_id_3_6),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id_4_8)
            == tree_node_id_4, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_node_id_4_8),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id_4_8)
            == tree_node_id_4, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_node_id_4_8),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id_5_7)
            == tree_node_id_5, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_node_id_5_7),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id_5_7)
            == list_node_id_5_5, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_node_id_5_7),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id_5_5)
            == list_node_id_5_7, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_node_id_5_5),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id_5_5)
            == tree_node_id_5, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_node_id_5_5),
            0);
        // Assert all insertion values.
        assert!(get_value_test(&avlq, list_node_id_3_9) == 9, 0);
        assert!(get_value_test(&avlq, list_node_id_3_6) == 6, 0);
        assert!(get_value_test(&avlq, list_node_id_4_8) == 8, 0);
        assert!(get_value_test(&avlq, list_node_id_5_7) == 7, 0);
        assert!(get_value_test(&avlq, list_node_id_5_5) == 5, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
        // Assert access keys flagged as such for descending AVL queue.
        avlq = new(DESCENDING, 0, 0);
        assert!(!is_ascending_access_key(insert(&mut avlq, 1, 0)), 0);
        assert!(!is_ascending_access_key(insert(&mut avlq, 2, 0)), 0);
        assert!(!is_ascending_access_key(insert(&mut avlq, 3, 0)), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns, state updates for `insert_check_eviction()`
    /// reference diagram case 1.
    fun test_insert_check_eviction_case_1() {
        let avlq = new(ASCENDING, 0, 0); // Initialize AVL queue.
        // Insert root tree node and node with insertion key 1.
        insert(&mut avlq, 1, 2);
        insert(&mut avlq, 1, 2);
        let v = 3; // Initialize insertion value counter.
        while (v <= N_NODES_MAX) {
            // Activate max list nodes.
            insert(&mut avlq, 3, v); // Insert with insertion value 3.
            v = v + 1; // Increment value counter.
        };
        // Attempt invalid insertion.
        let (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 3, 123, 2);
        // Assert flagged invalid.
        assert!(access_key == (NIL as u64), 0);
        assert!(evictee_access_key == (NIL as u64), 0);
        assert!(*option::borrow(&evictee_value) == 123, 0);
        // Attempt valid insertion.
        (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 2, 123, 2);
        // Assert access key lookup on insertion value.
        assert!(*borrow(&avlq, access_key) == 123, 0);
        // Assert encoded insertion key.
        assert!(get_access_key_insertion_key(access_key) == 2, 0);
        // Assert evictee insertion key.
        assert!(get_access_key_insertion_key(evictee_access_key) == 3, 0);
        // Assert evictee insertion value.
        assert!(*option::borrow(&evictee_value) == N_NODES_MAX, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns, state updates for `insert_check_eviction()`
    /// reference diagram case 2.
    fun test_insert_check_eviction_case_2() {
        let avlq = new(DESCENDING, 0, 0); // Initialize AVL queue.
        // Insert nodes top to bottom, left to right.
        insert(&mut avlq, 2, 123);
        insert(&mut avlq, 1, 456);
        insert(&mut avlq, 3, 789);
        insert(&mut avlq, 4, 321);
        // Attempt invalid insertion.
        let (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 1, 987, 1);
        // Assert flagged invalid.
        assert!(access_key == (NIL as u64), 0);
        assert!(evictee_access_key == (NIL as u64), 0);
        assert!(*option::borrow(&evictee_value) == 987, 0);
        // Attempt valid insertion.
        (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 2, 987, 1);
        // Assert access key lookup on insertion value.
        assert!(*borrow(&avlq, access_key) == 987, 0);
        // Assert encoded insertion key.
        assert!(get_access_key_insertion_key(access_key) == 2, 0);
        // Assert evictee insertion key.
        assert!(get_access_key_insertion_key(evictee_access_key) == 1, 0);
        // Assert evictee insertion value.
        assert!(*option::borrow(&evictee_value) == 456, 0);
        // Attempt valid insertion.
        (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 1, 654, 10);
        // Assert access key lookup on insertion value.
        assert!(*borrow(&avlq, access_key) == 654, 0);
        // Assert encoded insertion key.
        assert!(get_access_key_insertion_key(access_key) == 1, 0);
        // Assert no evictee.
        assert!(evictee_access_key == (NIL as u64), 0);
        assert!(option::is_none(&evictee_value), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns, state updates for inserting to empty AVL queue.
    fun test_insert_check_eviction_empty() {
        let avlq = new(ASCENDING, 0, 0); // Initialize AVL queue.
        let (access_key, evictee_access_key, evictee_value) =
            insert_check_eviction(&mut avlq, 123, 456, 0);
        // Assert access key lookup on insertion value.
        assert!(*borrow(&avlq, access_key) == 456, 0);
        // Assert encoded insertion key.
        assert!(get_access_key_insertion_key(access_key) == 123, 0);
        // Assert no evictee.
        assert!(evictee_access_key == (NIL as u64), 0);
        assert!(option::is_none(&evictee_value), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_HEIGHT)]
    /// Verify failure for invalid height.
    fun test_insert_check_eviction_invalid_height() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Initialize AVL queue.
        // Attempt invalid insertion.
        insert_check_eviction(&mut avlq, 1, 1, MAX_HEIGHT + 1);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful state manipulation.
    fun test_insert_check_head_tail_ascending() {
        // Init ascending AVL queue.
        let avlq = new<u8>(ASCENDING, 0, 0);
        // Assert head and tail fields.
        assert!(get_head_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_head_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_node_id_test(&avlq) == (NIL as u64), 0);
        // Declare insertion key and list node ID.
        let key_0 = HI_INSERTION_KEY - 1;
        let list_node_id_0 = HI_NODE_ID;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_0, list_node_id_0);
        // Assert head and tail fields both updated.
        assert!(get_head_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_0, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_0, 0);
        // Declare same insertion key with new node ID.
        let key_1 = key_0;
        let list_node_id_1 = list_node_id_0 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_1, list_node_id_1);
        // Assert head not updated, but tail updated.
        assert!(get_head_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_1, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_0, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_1, 0);
        // Declare insertion key smaller than first, new node ID.
        let key_2 = key_1 - 1;
        let list_node_id_2 = list_node_id_1 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_2, list_node_id_2);
        // Assert head updated, but tail not updated.
        assert!(get_head_key_test(&avlq) == key_2, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_2, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_1, 0);
        // Declare insertion key larger than first, new node ID.
        let key_3 = key_0 + 1;
        let list_node_id_3 = list_node_id_1 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_3, list_node_id_3);
        // Assert head not updated, but tail updated.
        assert!(get_head_key_test(&avlq) == key_2, 0);
        assert!(get_tail_key_test(&avlq) == key_3, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_2, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_3, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful state manipulation.
    fun test_insert_check_head_tail_descending() {
        // Init descending AVL queue.
        let avlq = new<u8>(DESCENDING, 0, 0);
        // Assert head and tail fields.
        assert!(get_head_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_head_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_node_id_test(&avlq) == (NIL as u64), 0);
        // Declare insertion key and list node ID.
        let key_0 = HI_INSERTION_KEY - 1;
        let list_node_id_0 = HI_NODE_ID;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_0, list_node_id_0);
        // Assert head and tail fields both updated.
        assert!(get_head_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_0, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_0, 0);
        // Declare same insertion key with new node ID.
        let key_1 = key_0;
        let list_node_id_1 = list_node_id_0 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_1, list_node_id_1);
        // Assert head not updated, but tail updated.
        assert!(get_head_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_tail_key_test(&avlq) == key_1, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_0, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_1, 0);
        // Declare insertion key larger than first, new node ID.
        let key_2 = key_1 + 1;
        let list_node_id_2 = list_node_id_1 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_2, list_node_id_2);
        // Assert head updated, but tail not updated.
        assert!(get_head_key_test(&avlq) == key_2, 0);
        assert!(get_tail_key_test(&avlq) == key_0, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_2, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_1, 0);
        // Declare insertion key smaller than first, new node ID.
        let key_3 = key_0 - 1;
        let list_node_id_3 = list_node_id_1 - 1;
        // Check head and tail accordingly.
        insert_check_head_tail(&mut avlq, key_3, list_node_id_3);
        // Assert head not updated, but tail updated.
        assert!(get_head_key_test(&avlq) == key_2, 0);
        assert!(get_tail_key_test(&avlq) == key_3, 0);
        assert!(get_head_node_id_test(&avlq) == list_node_id_2, 0);
        assert!(get_tail_node_id_test(&avlq) == list_node_id_3, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful returns for ascending and descending cases.
    fun test_insert_evict_tail() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Insert two elements in same doubly linked list, storing
        // access keys.
        let (access_key_2_123, access_key_2_456) = (
            insert(&mut avlq, 2, 123), insert(&mut avlq, 2, 456));
        // Insert and evict tail, storing returns.
        let (access_key_1_789, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 1, 789);
        // Assert returns.
        assert!(access_key_return == access_key_2_456, 0);
        assert!(insertion_value_return == 456, 0);
        // Insert and evict tail, storing returns.
        let (access_key_1_321, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 1, 321);
        // Assert returns.
        assert!(access_key_return == access_key_2_123, 0);
        assert!(insertion_value_return == 123, 0);
        // Insert and evict tail, storing returns.
        (_, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 0, 0);
        // Assert returns.
        assert!(access_key_return == access_key_1_321, 0);
        assert!(insertion_value_return == 321, 0);
        // Insert and evict tail, storing returns.
        (_, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 0, 0);
        // Assert returns.
        assert!(access_key_return == access_key_1_789, 0);
        assert!(insertion_value_return == 789, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
        avlq = new(DESCENDING, 0, 0); // Init AVL queue.
        // Insert two elements in same doubly linked list, storing
        // access keys.
        (access_key_2_123, access_key_2_456) = (
            insert(&mut avlq, 2, 123), insert(&mut avlq, 2, 456));
        // Insert and evict tail, storing returns.
        let (access_key_3_789, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 3, 789);
        // Assert returns.
        assert!(access_key_return == access_key_2_456, 0);
        assert!(insertion_value_return == 456, 0);
        // Insert and evict tail, storing returns.
        let (access_key_3_321, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 3, 321);
        // Assert returns.
        assert!(access_key_return == access_key_2_123, 0);
        assert!(insertion_value_return == 123, 0);
        // Insert and evict tail, storing returns.
        (_, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 4, 0);
        // Assert returns.
        assert!(access_key_return == access_key_3_321, 0);
        assert!(insertion_value_return == 321, 0);
        // Insert and evict tail, storing returns.
        (_, access_key_return, insertion_value_return) =
            insert_evict_tail(&mut avlq, 4, 0);
        // Assert returns.
        assert!(access_key_return == access_key_3_789, 0);
        assert!(insertion_value_return == 789, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_EVICT_EMPTY)]
    /// Verify failure for insertion with eviction from empty AVL queue.
    fun test_insert_evict_tail_empty() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Attempt invalid insertion with eviction.
        insert_evict_tail(&mut avlq, 0, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_EVICT_NEW_TAIL)]
    /// Verify failure for insertion with eviction for key-value
    /// insertion pair that would become tail.
    fun test_insert_evict_tail_new_tail_ascending() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        insert(&mut avlq, 0, 0); // Insert tail.
        insert(&mut avlq, 1, 0); // Insert new tail.
        // Attempt invalid insertion with eviction.
        insert_evict_tail(&mut avlq, 1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_EVICT_NEW_TAIL)]
    /// Verify failure for insertion with eviction for key-value
    /// insertion pair that would become tail.
    fun test_insert_evict_tail_new_tail_descending() {
        let avlq = new(DESCENDING, 0, 0); // Init AVL queue.
        insert(&mut avlq, 1, 0); // Insert tail.
        insert(&mut avlq, 0, 0); // Insert new tail.
        // Attempt invalid insertion with eviction.
        insert_evict_tail(&mut avlq, 0, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_INSERTION_KEY_TOO_LARGE)]
    /// Verify failure for insertion key too large.
    fun test_insert_insertion_key_too_large() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Attempt invalid insertion
        insert(&mut avlq, HI_INSERTION_KEY + 1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify return and state updates for allocating new list node.
    fun test_insert_list_node_assign_fields_allocate() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Declare inputs.
        let value = 123;
        let last = 456;
        let next = 789;
        // Assign fields to inserted list node, store its ID.
        let list_node_id = insert_list_node_assign_fields(
            &mut avlq, last, next, value);
        assert!(list_node_id == 1, 0); // Assert list node ID.
        // Assert field assignments.
        let list_node_ref = borrow_list_node_test(&avlq, list_node_id);
        let (last_assigned, _) = get_list_last_test(list_node_ref);
        assert!(last_assigned == last, 0);
        let (next_assigned, _) = get_list_next_test(list_node_ref);
        assert!(next_assigned == next, 0);
        assert!(get_value_test(&avlq, list_node_id) == value, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify return and state updates for inserting stack top.
    fun test_insert_list_node_assign_fields_stacked() {
        let stack_top_id = 321;
        let avlq = new(ASCENDING, 0, stack_top_id); // Init AVL queue.
        // Declare inputs.
        let value = 123;
        let last = 456;
        let next = 789;
        // Assign fields to inserted list node, store its ID.
        let list_node_id = insert_list_node_assign_fields(
            &mut avlq, last, next, value);
        // Assert list node ID.
        assert!(list_node_id == stack_top_id, 0);
        // Assert field assignments.
        let list_node_ref = borrow_list_node_test(&avlq, list_node_id);
        let (last_assigned, _) = get_list_last_test(list_node_ref);
        assert!(last_assigned == last, 0);
        let (next_assigned, _) = get_list_next_test(list_node_ref);
        assert!(next_assigned == next, 0);
        assert!(get_value_test(&avlq, list_node_id) == value, 0);
        // Assert stack top update.
        assert!(get_list_top_test(&avlq) == stack_top_id - 1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns for list node becoming new tail.
    fun test_insert_list_node_get_last_next_new_tail() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        let anchor_tree_node_id = 15; // Declare anchor tree node ID.
        let old_list_tail = 31; // Declare old list tail node ID.
        // Manually add anchor tree node to tree nodes table.
        table_with_length::add(&mut avlq.tree_nodes, anchor_tree_node_id,
            TreeNode { bits: (old_list_tail as u128) << SHIFT_LIST_TAIL });
        let (last, next) = // Get virtual last and next fields.
            insert_list_node_get_last_next(&avlq, anchor_tree_node_id);
        // Assert last and next fields.
        assert!(last == u_64(b"11111"), 0);
        assert!(next == u_64(b"100000000001111"), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns for solo list node and allocated tree node.
    fun test_insert_list_node_get_last_next_solo_allocate() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        let (last, next) = // Get virtual last and next fields.
            insert_list_node_get_last_next(&avlq, (NIL as u64));
        // Assert last and next fields.
        assert!(last == u_64(b"100000000000001"), 0);
        assert!(next == u_64(b"100000000000001"), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns for solo list node and tree node on stack.
    fun test_insert_list_node_get_last_next_solo_stacked() {
        let avlq = new<u8>(ASCENDING, 7, 0); // Init AVL queue.
        let (last, next) = // Get virtual last and next fields.
            insert_list_node_get_last_next(&avlq, (NIL as u64));
        // Assert last and next fields.
        assert!(last == u_64(b"100000000000111"), 0);
        assert!(next == u_64(b"100000000000111"), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify return, state updates for list node that is not solo.
    fun test_insert_list_node_not_solo() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        // Declare old list tail state.
        let old_list_tail = 1;
        let anchor_tree_node_id = 321;
        let list_node_id = 2; // Declare list node ID post-allocation.
        // Manually add anchor tree node to tree nodes table.
        table_with_length::add(&mut avlq.tree_nodes, anchor_tree_node_id,
            TreeNode { bits: (old_list_tail as u128) << SHIFT_LIST_TAIL });
        // Manually add old list tail to list nodes table.
        table_with_length::add(&mut avlq.list_nodes, old_list_tail,
            ListNode { last_msbs: 0, last_lsbs: 0, next_msbs: 0, next_lsbs: 0 });
        let value = 100; // Declare insertion value.
        let list_node_id_return = // Insert node, storing resultant ID.
            insert_list_node(&mut avlq, anchor_tree_node_id, value);
        // Assert return.
        assert!(list_node_id_return == list_node_id, 0);
        // Assert state updates.
        let list_node_ref = borrow_list_node_test(&avlq, list_node_id);
        let (last_assigned, is_tree_node) = get_list_last_test(list_node_ref);
        assert!(last_assigned == old_list_tail, 0);
        assert!(!is_tree_node, 0);
        let (next_assigned, is_tree_node) = get_list_next_test(list_node_ref);
        assert!(next_assigned == anchor_tree_node_id, 0);
        assert!(is_tree_node, 0);
        let old_tail_ref = borrow_list_node_test(&avlq, old_list_tail);
        (next_assigned, is_tree_node) = get_list_next_test(old_tail_ref);
        assert!(next_assigned == list_node_id, 0);
        assert!(!is_tree_node, 0);
        assert!(get_value_test(&avlq, list_node_id) == value, 0);
        let anchor_node_ref =
            borrow_tree_node_test(&avlq, anchor_tree_node_id);
        assert!(get_list_tail_test(anchor_node_ref) == list_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify return, state updates for solo list node.
    fun test_insert_list_node_solo() {
        // Declare tree node ID and list node IDs at top of inactive
        // stacks.
        let tree_node_id = 123;
        let list_node_id = 456;
        // Init AVL queue.
        let avlq = new(ASCENDING, tree_node_id, list_node_id);
        let value = 100; // Declare insertion value.
        let list_node_id_return = // Insert node, storing resultant ID.
            insert_list_node(&mut avlq, (NIL as u64), value);
        // Assert return.
        assert!(list_node_id_return == list_node_id, 0);
        // Assert state updates.
        let list_node_ref = borrow_list_node_test(&avlq, list_node_id);
        let (last_assigned, is_tree_node) = get_list_last_test(list_node_ref);
        assert!(last_assigned == tree_node_id, 0);
        assert!(is_tree_node, 0);
        let (next_assigned, is_tree_node) = get_list_next_test(list_node_ref);
        assert!(next_assigned == tree_node_id, 0);
        assert!(is_tree_node, 0);
        assert!(get_value_test(&avlq, list_node_id) == value, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_TOO_MANY_LIST_NODES)]
    /// Assert failure for too many list nodes.
    fun test_insert_too_many_list_nodes() {
        // Init AVL queue with max list nodes allocated.
        let avlq = new(ASCENDING, 0, N_NODES_MAX);
        // Reassign inactive list nodes stack top to null:
        avlq.bits = avlq.bits &
            (HI_128 ^ // Clear out field via mask unset at field bits.
                (((HI_NODE_ID as u128) << SHIFT_LIST_STACK_TOP) as u128));
        // Attempt invalid insertion.
        insert(&mut avlq, 0, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state update for inserting tree node with empty stack.
    fun test_insert_tree_node_empty() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        let tree_node_id = 1; // Declare inserted tree node ID.
        let solo_node_id = 789; // Declare solo list node ID.
        let key = 321; // Declare insertion key.
        // Insert new tree node, storing its tree node ID.
        let tree_node_id_return = insert_tree_node(
            &mut avlq, key, (NIL as u64), solo_node_id, option::none());
        // Assert inserted tree node ID.
        assert!(tree_node_id_return == tree_node_id, 0);
        // Assert new tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id) == key, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id) == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id)
            == solo_node_id, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id)
            == solo_node_id, 0);
        // Assert stack top.
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        // Assert root update.
        assert!(get_root_test(&avlq) == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state update for inserting tree node with stack.
    fun test_insert_tree_node_stacked() {
        let tree_node_id = 123; // Declare inserted tree node ID.
        // Init AVL queue.
        let avlq = new<u8>(ASCENDING, tree_node_id, 0);
        let solo_node_id = 789; // Declare solo list node ID.
        let key = 321; // Declare insertion key.
        // Insert tree node, storing its tree node ID.
        let tree_node_id_return = insert_tree_node(
            &mut avlq, key, (NIL as u64), solo_node_id, option::none());
        // Assert inserted tree node ID.
        assert!(tree_node_id_return == tree_node_id, 0);
        // Assert tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id) == key, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id) == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id)
            == solo_node_id, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id)
            == solo_node_id, 0);
        // Assert stack top.
        assert!(get_tree_top_test(&avlq) == tree_node_id - 1, 0);
        // Assert root update.
        assert!(get_root_test(&avlq) == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state update for inserting left child.
    fun test_insert_tree_node_update_parent_edge_left() {
        let tree_node_id = 1234; // Declare inserted tree node ID.
        let parent = 321;
        let avlq = new<u8>(ASCENDING, parent, 0); // Init AVL queue.
        // Declare empty new leaf side.
        let new_leaf_side = option::some(LEFT);
        // Update parent to inserted node.
        insert_tree_node_update_parent_edge(
            &mut avlq, tree_node_id, parent, new_leaf_side);
        // Assert update to parent's child field.
        assert!(get_child_left_by_id_test(&avlq, parent) == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state update for inserting right child.
    fun test_insert_tree_node_update_parent_edge_right() {
        let tree_node_id = 1234; // Declare inserted tree node ID.
        let parent = 321;
        let avlq = new<u8>(ASCENDING, parent, 0); // Init AVL queue.
        // Declare empty new leaf side.
        let new_leaf_side = option::some(RIGHT);
        // Update parent to inserted node.
        insert_tree_node_update_parent_edge(
            &mut avlq, tree_node_id, parent, new_leaf_side);
        // Assert update to parent's child field.
        assert!(get_child_right_by_id_test(&avlq, parent) == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state update for inserting root.
    fun test_insert_tree_node_update_parent_edge_root() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        let tree_node_id = 1234; // Declare inserted tree node ID.
        let parent = (NIL as u64); // Declare parent as root flag.
        // Declare empty new leaf side.
        let new_leaf_side = option::none();
        // Assert null root.
        assert!(get_root_test(&avlq) == (NIL as u64), 0);
        // Update parent for inserted root node.
        insert_tree_node_update_parent_edge(
            &mut avlq, tree_node_id, parent, new_leaf_side);
        // Assert root update.
        assert!(get_root_test(&avlq) == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful checks.
    fun test_is_ascending() {
        let avlq = AVLqueue<u8> {
            // Create empty AVL queue.
            bits: (NIL as u128),
            root_lsbs: NIL,
            tree_nodes: table_with_length::new(),
            list_nodes: table_with_length::new(),
            values: table::new(),
        };
        // Assert flagged descending.
        assert!(!is_ascending(&avlq), 0);
        // Flag as ascending.
        avlq.bits = u_128_by_32(
            b"01000000000000000000000000000000",
            // ^ bit 126
            b"00000000000000000000000000000000",
            b"00000000000000000000000000000000",
            b"00000000000000000000000000000000"
        );
        // Assert flagged descending.
        assert!(is_ascending(&avlq), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful returns.
    fun test_is_empty() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        assert!(is_empty(&avlq), 0); // Assert marked empty.
        let access_key = insert(&mut avlq, 0, 0); // Insert.
        assert!(!is_empty(&avlq), 0); // Assert marked not empty.
        remove(&mut avlq, access_key); // Remove sole entry.
        assert!(is_empty(&avlq), 0); // Assert marked empty.
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful initialization for no node allocations.
    fun test_new_no_nodes() {
        // Init ascending AVL queue.
        let avlq = new<u8>(ASCENDING, 0, 0);
        // Assert flagged ascending.
        assert!(is_ascending(&avlq), 0);
        // Assert null stack tops.
        assert!(get_list_top_test(&avlq) == (NIL as u64), 0);
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
        // Init descending AVL queue.
        avlq = new(DESCENDING, 0, 0);
        // Assert flagged descending.
        assert!(!is_ascending(&avlq), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful initialization for allocating tree nodes.
    fun test_new_some_nodes() {
        // Init ascending AVL queue with two nodes each.
        let avlq = new<u8>(ASCENDING, 3, 2);
        // Assert table lengths.
        assert!(table_with_length::length(&avlq.tree_nodes) == 3, 0);
        assert!(table_with_length::length(&avlq.list_nodes) == 2, 0);
        // Assert stack tops.
        assert!(get_tree_top_test(&avlq) == 3, 0);
        assert!(get_list_top_test(&avlq) == 2, 0);
        // Assert inactive tree node stack next chain.
        assert!(get_tree_next_test(borrow_tree_node_test(&avlq, 3)) == 2, 0);
        assert!(get_tree_next_test(borrow_tree_node_test(&avlq, 2)) == 1, 0);
        assert!(get_tree_next_test(borrow_tree_node_test(&avlq, 1)) ==
            (NIL as u64), 0);
        // Assert inactive list node stack next chain.
        let (node_id, is_tree_node) =
            get_list_next_test(borrow_list_node_test(&avlq, 2));
        assert!(node_id == 1, 0);
        assert!(!is_tree_node, 0);
        (node_id, is_tree_node) =
            get_list_next_test(borrow_list_node_test(&avlq, 1));
        assert!(node_id == (NIL as u64), 0);
        assert!(!is_tree_node, 0);
        // Assert value options initialize to none.
        assert!(option::is_none(borrow_value_option_test(&avlq, 2)), 0);
        assert!(option::is_none(borrow_value_option_test(&avlq, 1)), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful initialization for allocating tree nodes.
    fun test_new_some_nodes_loop() {
        // Declare number of tree and list nodes to allocate.
        let (n_tree_nodes, n_list_nodes) = (1234, 321);
        // Init ascending AVL queue accordingly.
        let avlq = new<u8>(ASCENDING, n_tree_nodes, n_list_nodes);
        // Assert table lengths.
        assert!(table_with_length::length(&avlq.tree_nodes) ==
            n_tree_nodes, 0);
        assert!(table_with_length::length(&avlq.list_nodes) ==
            n_list_nodes, 0);
        // Assert stack tops.
        assert!(get_tree_top_test(&avlq) == n_tree_nodes, 0);
        assert!(get_list_top_test(&avlq) == n_list_nodes, 0);
        let i = n_tree_nodes; // Declare loop counter.
        while (i > (NIL as u64)) {
            // Loop over all tree nodes in stack:
            // Assert next indicated tree node in stack.
            assert!(get_tree_next_test(borrow_tree_node_test(&avlq, i)) ==
                i - 1, 0);
            i = i - 1; // Decrement loop counter.
        };
        i = n_list_nodes; // Re-declare loop counter.
        while (i > (NIL as u64)) {
            // Loop over all list nodes in stack:
            // Assert next indicated list node in stack.
            let (node_id, is_tree_node) =
                get_list_next_test(borrow_list_node_test(&avlq, i));
            assert!(node_id == i - 1, 0);
            assert!(!is_tree_node, 0);
            // Assert value option initializes to none.
            assert!(option::is_none(borrow_value_option_test(&avlq, i)), 0);
            i = i - 1; // Decrement loop counter.
        };
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_TOO_MANY_LIST_NODES)]
    /// Verify failure for attempting to allocate too many list nodes.
    fun test_new_too_many_list_nodes() {
        // Attempt invalid invocation.
        let avlq = new<u8>(ASCENDING, 0, N_NODES_MAX + 1);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_TOO_MANY_TREE_NODES)]
    /// Verify failure for attempting to allocate too many tree nodes.
    fun test_new_too_many_tree_nodes() {
        // Attempt invalid invocation.
        let avlq = new<u8>(ASCENDING, N_NODES_MAX + 1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns for ascending and descending AVL queue.
    fun test_next_list_node_id_in_access_key() {
        let n_list_nodes = 15;
        let list_nodes_per_tree_node = 3;
        let avlq = new(ASCENDING, 0, 0);
        let access_keys = vector[];
        let i = 0;
        // Insert multiple list nodes per tree node.
        while (i < n_list_nodes) {
            vector::push_back(
                &mut access_keys,
                insert(&mut avlq, i / list_nodes_per_tree_node, i)
            );
            i = i + 1;
        };
        let list_node_id_in_access_key = *vector::borrow(&access_keys, 0);
        i = 0;
        // Assert next operation for all nodes except last.
        while (i < (n_list_nodes - 1)) {
            list_node_id_in_access_key = next_list_node_id_in_access_key(
                &avlq,
                list_node_id_in_access_key
            );
            assert!(*borrow(&avlq, list_node_id_in_access_key) == i + 1, 0);
            i = i + 1;
        };
        // Assert operation for last node.
        assert!(
            (NIL as u64) == next_list_node_id_in_access_key(
                &avlq,
                list_node_id_in_access_key
            ),
            0
        );
        drop_avlq_test(avlq); // Drop AVL queue.
        // Repeat for descending AVL queue.
        avlq = new(DESCENDING, 0, 0);
        i = 0;
        // Insert multiple list nodes per tree node, descending queue.
        while (i < n_list_nodes) {
            vector::push_back(
                &mut access_keys,
                insert(
                    &mut avlq,
                    HI_INSERTION_KEY - i / list_nodes_per_tree_node, i
                )
            );
            i = i + 1;
        };
        list_node_id_in_access_key = *vector::borrow(&access_keys, 0);
        i = 0;
        // Assert next operation for all nodes except last.
        while (i < (n_list_nodes - 1)) {
            list_node_id_in_access_key = next_list_node_id_in_access_key(
                &avlq,
                list_node_id_in_access_key
            );
            assert!(*borrow(&avlq, list_node_id_in_access_key) == i + 1, 0);
            i = i + 1;
        };
        // Assert operation for last node.
        assert!(
            (NIL as u64) == next_list_node_id_in_access_key(
                &avlq,
                list_node_id_in_access_key
            ),
            0
        );
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Insert 5 key-value pairs for each of 80 insertion keys, pop
    /// first 200 from head, then final 200 from tail.
    fun test_pop_head_tail() {
        let avlq = new(ASCENDING, 0, 0); // Init AVL queue.
        let i = 0; // Declare loop counter.
        while (i < 400) {
            // Insert all key-value insertion pairs.
            // {0, 0}, {0, 1}, ... {0, 4}, {1, 5}, ... {79, 399}
            insert(&mut avlq, i / 5, i); // Insert all keys.
            i = i + 1; // Increment loop counter.
        };
        i = 0; // Reset loop counter.
        while (i < 200) {
            // Pop first 200 from head.
            // Assert popped insertion value.
            assert!(pop_head(&mut avlq) == i, 0);
            i = i + 1; // Increment loop counter.
        };
        i = 0; // Reset loop counter.
        while (i < 200) {
            // Pop final 200 from tail.
            // Assert popped insertion value.
            assert!(pop_tail(&mut avlq) == 399 - i, 0);
            i = i + 1; // Increment loop counter.
        };
        assert!(is_empty(&avlq), 0); // Assert AVL queue empty.
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove()` reference diagram case 1.
    fun test_remove_1() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert tree nodes top to bottom, list nodes head to tail.
        let (access_key_2_3, access_key_2_4, access_key_1_5, access_key_1_6) =
            (insert(&mut avlq, 2, 3), insert(&mut avlq, 2, 4),
                insert(&mut avlq, 1, 5), insert(&mut avlq, 1, 6));
        // Get node IDs.
        let node_id_1 = get_access_key_tree_node_id_test(access_key_1_5);
        let node_id_2 = get_access_key_tree_node_id_test(access_key_2_3);
        let node_id_3 = get_access_key_list_node_id_test(access_key_2_3);
        let node_id_4 = get_access_key_list_node_id_test(access_key_2_4);
        let node_id_5 = get_access_key_list_node_id_test(access_key_1_5);
        let node_id_6 = get_access_key_list_node_id_test(access_key_1_6);
        // Execute first removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_5) == 5, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        assert!(get_list_top_test(&avlq) == node_id_5, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_6, 0);
        assert!(get_head_key_test(&avlq) == 1, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_4, 0);
        assert!(get_tail_key_test(&avlq) == 2, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_4, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == node_id_3, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        // Execute second removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_6) == 6, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_1, 0);
        assert!(get_list_top_test(&avlq) == node_id_6, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_3, 0);
        assert!(get_head_key_test(&avlq) == 2, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_4, 0);
        assert!(get_tail_key_test(&avlq) == 2, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_1_ref = borrow_tree_node_test(&avlq, node_id_1);
        assert!(node_1_ref.bits == (NIL as u128), 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_5, 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_4, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == node_id_3, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove()` reference diagram case 2.
    fun test_remove_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert tree nodes top to bottom, list nodes head to tail.
        let (access_key_2_3, access_key_2_4, access_key_1_5, access_key_1_6) =
            (insert(&mut avlq, 2, 3), insert(&mut avlq, 2, 4),
                insert(&mut avlq, 1, 5), insert(&mut avlq, 1, 6));
        // Get node IDs.
        let node_id_1 = get_access_key_tree_node_id_test(access_key_1_5);
        let node_id_2 = get_access_key_tree_node_id_test(access_key_2_3);
        let node_id_3 = get_access_key_list_node_id_test(access_key_2_3);
        let node_id_4 = get_access_key_list_node_id_test(access_key_2_4);
        let node_id_5 = get_access_key_list_node_id_test(access_key_1_5);
        let node_id_6 = get_access_key_list_node_id_test(access_key_1_6);
        // Assert local tail state for tree node with insertion key 2.
        assert!(!is_local_tail(&avlq, access_key_2_3), 0);
        assert!(is_local_tail(&avlq, access_key_2_4), 0);
        // Execute first removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_2_4) == 4, 0);
        // Assert local tail state for tree node with insertion key 2.
        assert!(is_local_tail(&avlq, access_key_2_3), 0);
        assert!(!is_local_tail(&avlq, access_key_2_4), 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        assert!(get_list_top_test(&avlq) == node_id_4, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_5, 0);
        assert!(get_head_key_test(&avlq) == 1, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_3, 0);
        assert!(get_tail_key_test(&avlq) == 2, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_6, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == node_id_5, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        // Execute second removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_2_3) == 3, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_2, 0);
        assert!(get_list_top_test(&avlq) == node_id_3, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_5, 0);
        assert!(get_head_key_test(&avlq) == 1, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_6, 0);
        assert!(get_tail_key_test(&avlq) == 1, 0);
        assert!(get_root_test(&avlq) == node_id_1, 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_2_ref = borrow_tree_node_test(&avlq, node_id_2);
        assert!(node_2_ref.bits == (NIL as u128), 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_4, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_6, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == node_id_5, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove()` reference diagram case 3.
    fun test_remove_3() {
        let avlq = new<u8>(DESCENDING, 0, 0); // Init AVL queue.
        // Insert tree nodes top to bottom, list nodes head to tail.
        let (access_key_2_3, access_key_2_4, access_key_1_5, access_key_1_6) =
            (insert(&mut avlq, 2, 3), insert(&mut avlq, 2, 4),
                insert(&mut avlq, 1, 5), insert(&mut avlq, 1, 6));
        // Get node IDs.
        let node_id_1 = get_access_key_tree_node_id_test(access_key_1_5);
        let node_id_2 = get_access_key_tree_node_id_test(access_key_2_3);
        let node_id_3 = get_access_key_list_node_id_test(access_key_2_3);
        let node_id_4 = get_access_key_list_node_id_test(access_key_2_4);
        let node_id_5 = get_access_key_list_node_id_test(access_key_1_5);
        let node_id_6 = get_access_key_list_node_id_test(access_key_1_6);
        // Execute first removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_2_3) == 3, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        assert!(get_list_top_test(&avlq) == node_id_3, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_4, 0);
        assert!(get_head_key_test(&avlq) == 2, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_6, 0);
        assert!(get_tail_key_test(&avlq) == 1, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_6, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == node_id_5, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        // Execute second removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_2_4) == 4, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_2, 0);
        assert!(get_list_top_test(&avlq) == node_id_4, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_5, 0);
        assert!(get_head_key_test(&avlq) == 1, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_6, 0);
        assert!(get_tail_key_test(&avlq) == 1, 0);
        assert!(get_root_test(&avlq) == node_id_1, 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_2_ref = borrow_tree_node_test(&avlq, node_id_2);
        assert!(node_2_ref.bits == (NIL as u128), 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_6, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_3, 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_6, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == node_id_5, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == node_id_1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove()` reference diagram case 4.
    fun test_remove_4() {
        let avlq = new<u8>(DESCENDING, 0, 0); // Init AVL queue.
        // Insert tree nodes top to bottom, list nodes head to tail.
        let (access_key_2_3, access_key_2_4, access_key_1_5, access_key_1_6) =
            (insert(&mut avlq, 2, 3), insert(&mut avlq, 2, 4),
                insert(&mut avlq, 1, 5), insert(&mut avlq, 1, 6));
        // Get node IDs.
        let node_id_1 = get_access_key_tree_node_id_test(access_key_1_5);
        let node_id_2 = get_access_key_tree_node_id_test(access_key_2_3);
        let node_id_3 = get_access_key_list_node_id_test(access_key_2_3);
        let node_id_4 = get_access_key_list_node_id_test(access_key_2_4);
        let node_id_5 = get_access_key_list_node_id_test(access_key_1_5);
        let node_id_6 = get_access_key_list_node_id_test(access_key_1_6);
        // Execute first removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_6) == 6, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == (NIL as u64), 0);
        assert!(get_list_top_test(&avlq) == node_id_6, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_3, 0);
        assert!(get_head_key_test(&avlq) == 2, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_5, 0);
        assert!(get_tail_key_test(&avlq) == 1, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_1)
            == node_id_5, 0);
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_1, 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_4, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == node_id_3, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        // Execute second removal, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_5) == 5, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_1, 0);
        assert!(get_list_top_test(&avlq) == node_id_5, 0);
        assert!(get_head_node_id_test(&avlq) == node_id_3, 0);
        assert!(get_head_key_test(&avlq) == 2, 0);
        assert!(get_tail_node_id_test(&avlq) == node_id_4, 0);
        assert!(get_tail_key_test(&avlq) == 2, 0);
        assert!(get_root_test(&avlq) == node_id_2, 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_1_ref = borrow_tree_node_test(&avlq, node_id_1);
        assert!(node_1_ref.bits == (NIL as u128), 0);
        // Assert active tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        assert!(get_list_tail_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_6), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_5), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_5)
            == node_id_6, 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_3)
            == node_id_4, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_4)
            == node_id_3, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, node_id_4), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove_tree_node_with_children()`
    /// reference diagram 1.
    fun test_remove_children_1() {
        // Declare number of allocated tree nodes.
        let n_allocated_tree_nodes = 8;
        // Init AVL queue with 4 allocated tree nodes.
        let avlq = new<u8>(ASCENDING, n_allocated_tree_nodes, 0);
        // Insert nodes from top to bottom, left to right.
        let (node_id_2, node_id_1, node_id_4, node_id_3, node_id_5) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)));
        // Declare number of active tree nodes.
        let n_active_tree_nodes = 5;
        remove_tree_node(&mut avlq, node_id_4); // Remove node x.
        assert!(get_root_test(&avlq) == node_id_2, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_id_4, 0);
        // Assert node x state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_4_ref = borrow_tree_node_test(&avlq, node_id_4);
        assert!(node_4_ref.bits ==
            ((n_allocated_tree_nodes - n_active_tree_nodes) as u128), 0);
        // Assert node l state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_3) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_3) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_3) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_3)
            == node_id_5, 0);
        // Assert node r state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_5) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_5) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_5) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_5)
            == node_id_3, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_5)
            == (NIL as u64), 0);
        // Assert node 1 state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        // Assert node 2 state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove_tree_node_with_children()`
    /// reference diagram 2.
    fun test_remove_children_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert nodes from top to bottom, left to right.
        let (node_id_5, node_id_2, node_id_6, node_id_1, node_id_4, node_id_7,
            node_id_3) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 6, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 7, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)));
        remove_tree_node(&mut avlq, node_id_5); // Remove node x.
        assert!(get_root_test(&avlq) == node_id_4, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_id_5, 0);
        // Assert node x state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_5_ref = borrow_tree_node_test(&avlq, node_id_5);
        assert!(node_5_ref.bits == (NIL as u128), 0);
        // Assert node r state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_6) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_6) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_6) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_6)
            == node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_6)
            == node_id_7, 0);
        // Assert node y state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_4) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_4) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_4) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_4)
            == node_id_6, 0);
        // Assert state for parent to node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        // Assert tree y state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_3) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_3) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_3) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        // Assert tree l state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        // Assert state for child to node r.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_7) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_7) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_7) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_7)
            == node_id_6, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_7)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `remove_tree_node_with_children()`
    /// reference diagram 3.
    fun test_remove_children_3() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert nodes from top to bottom, left to right.
        let (node_id_5, node_id_2, node_id_6, node_id_1, node_id_3, node_id_7,
            node_id_4) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 6, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 7, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)));
        remove_tree_node(&mut avlq, node_id_5); // Remove node x.
        assert!(get_root_test(&avlq) == node_id_4, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_id_5, 0);
        // Assert node x state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_5_ref = borrow_tree_node_test(&avlq, node_id_5);
        assert!(node_5_ref.bits == (NIL as u128), 0);
        // Assert node r state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_6) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_6) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_6) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_6)
            == node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_6)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_6)
            == node_id_7, 0);
        // Assert node y state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_4) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_4) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_4) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_4)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_4)
            == node_id_2, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_4)
            == node_id_6, 0);
        // Assert state for node l.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == node_id_4, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == node_id_1, 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == node_id_3, 0);
        // Assert state for parent to node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_3) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_3) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_3) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_3)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_3)
            == (NIL as u64), 0);
        // Assert tree l state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_1) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_1) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_1)
            == node_id_2, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        // Assert state for child to node r.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_7) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_7) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_7) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_7)
            == node_id_6, 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_7)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_1)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for removing head, tail, and sole list
    /// node.
    fun test_remove_list_node() {
        // Declare tree node ID for sole activated tree node.
        let tree_id_1 = 10;
        let avlq = new<u8>(ASCENDING, tree_id_1, 0); // Init AVL queue.
        // Insert, storing list node IDs.
        let list_id_1 = get_access_key_list_node_id_test(
            insert(&mut avlq, HI_INSERTION_KEY, 1));
        let list_id_2 = get_access_key_list_node_id_test(
            insert(&mut avlq, HI_INSERTION_KEY, 2));
        let list_id_3 = get_access_key_list_node_id_test(
            insert(&mut avlq, HI_INSERTION_KEY, 3));
        // Assert inactive list node stack top.
        assert!(get_list_top_test(&avlq) == (NIL as u64), 0);
        // Assert tree node state.
        assert!(get_list_head_by_id_test(&avlq, tree_id_1)
            == list_id_1, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_id_1)
            == list_id_3, 0);
        // Assert list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_1)
            == tree_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_1)
            == list_id_2, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_2)
            == list_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_2)
            == list_id_3, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_3)
            == list_id_2, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_3)
            == tree_id_1, 0);
        let (value, new_head, new_tail) = // Remove list head.
            remove_list_node(&mut avlq, list_id_1);
        // Assert returns.
        assert!(value == 1, 0);
        assert!(*option::borrow(&new_head) == list_id_2, 0);
        assert!(option::is_none(&new_tail), 0);
        // Assert inactive list node stack top.
        assert!(get_list_top_test(&avlq) == list_id_1, 0);
        // Assert tree node state.
        assert!(get_list_head_by_id_test(&avlq, tree_id_1)
            == list_id_2, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_id_1)
            == list_id_3, 0);
        // Assert list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_2)
            == tree_id_1, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_2)
            == list_id_3, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_3)
            == list_id_2, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_3)
            == tree_id_1, 0);
        // Remove list tail
        (value, new_head, new_tail) = remove_list_node(&mut avlq, list_id_3);
        // Assert returns.
        assert!(value == 3, 0);
        assert!(option::is_none(&new_head), 0);
        assert!(*option::borrow(&new_tail) == list_id_2, 0);
        // Assert inactive list node stack top.
        assert!(get_list_top_test(&avlq) == list_id_3, 0);
        // Assert tree node state.
        assert!(get_list_head_by_id_test(&avlq, tree_id_1)
            == list_id_2, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_id_1)
            == list_id_2, 0);
        // Assert list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(is_tree_node_list_last_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_2)
            == tree_id_1, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_2)
            == tree_id_1, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_3)
            == list_id_1, 0);
        // Remove sole node in list
        (value, new_head, new_tail) = remove_list_node(&mut avlq, list_id_2);
        // Assert returns.
        assert!(value == 2, 0);
        assert!(*option::borrow(&new_head) == (NIL as u64), 0);
        assert!(*option::borrow(&new_tail) == (NIL as u64), 0);
        // Assert inactive list node stack top.
        assert!(get_list_top_test(&avlq) == list_id_2, 0);
        // Assert tree node state unmodified.
        assert!(get_list_head_by_id_test(&avlq, tree_id_1)
            == list_id_2, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_id_1)
            == list_id_2, 0);
        // Assert list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_1), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_1)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_2)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_2)
            == list_id_3, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_id_3)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_id_3), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_id_3)
            == list_id_1, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for removing a list node that is neither
    /// head nor tail of corresponding doubly linked list.
    fun test_remove_mid_list() {
        let (n_allocated_tree_nodes, n_allocated_list_nodes) = (7, 4);
        let avlq = new<u8>(// Init AVL queue.
            ASCENDING, n_allocated_tree_nodes, n_allocated_list_nodes);
        // Declare three list nodes all having insertion key 1.
        let access_key_head = insert(&mut avlq, 1, 1);
        let access_key_middle = insert(&mut avlq, 1, 2);
        let access_key_tail = insert(&mut avlq, 1, 3);
        // Remove node from middle of list, asserting insertion value.
        assert!(remove(&mut avlq, access_key_middle) == 2, 0);
        let head_list_node_id = // Get head list node ID.
            get_access_key_list_node_id_test(access_key_head);
        let tail_list_node_id = // Get tail list node ID.
            get_access_key_list_node_id_test(access_key_tail);
        let list_node_id = // Get removed list node ID.
            get_access_key_list_node_id_test(access_key_middle);
        let tree_node_id = // Get active tree node ID.
            get_access_key_tree_node_id_test(access_key_head);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == n_allocated_tree_nodes - 1, 0);
        assert!(get_list_top_test(&avlq) == list_node_id, 0);
        assert!(get_head_node_id_test(&avlq) == head_list_node_id, 0);
        assert!(get_head_key_test(&avlq) == 1, 0);
        assert!(get_tail_node_id_test(&avlq) == tail_list_node_id, 0);
        assert!(get_tail_key_test(&avlq) == 1, 0);
        assert!(get_root_test(&avlq) == tree_node_id, 0);
        // Assert tree node state.
        assert!(get_insertion_key_by_id_test(&avlq, tree_node_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_node_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_node_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_node_id)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, tree_node_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_node_id)
            == (NIL as u64), 0);
        assert!(get_list_head_by_id_test(&avlq, tree_node_id)
            == head_list_node_id, 0);
        assert!(get_list_tail_by_id_test(&avlq, tree_node_id)
            == tail_list_node_id, 0);
        // Assert inactive list node state.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, list_node_id), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, list_node_id)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, list_node_id), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, list_node_id)
            == n_allocated_list_nodes - 3, 0);
        // Assert active list node state.
        assert!(is_tree_node_list_last_by_id_test(&avlq, head_list_node_id),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, head_list_node_id)
            == tree_node_id, 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, head_list_node_id),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, head_list_node_id)
            == tail_list_node_id, 0);
        assert!(!is_tree_node_list_last_by_id_test(&avlq, tail_list_node_id),
            0);
        assert!(get_list_last_node_id_by_id_test(&avlq, tail_list_node_id)
            == head_list_node_id, 0);
        assert!(is_tree_node_list_next_by_id_test(&avlq, tail_list_node_id),
            0);
        assert!(get_list_next_node_id_by_id_test(&avlq, tail_list_node_id)
            == tree_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for removing key-value insertion pair
    /// (1, 2) as sole entry in ascending as descending AVL queue.
    fun test_remove_root() {
        // Init ascending AVL queue.
        let avlq = new<u8>(ASCENDING, 0, 0);
        // Insert sole entry.
        let access_key_1_2 = insert(&mut avlq, 1, 2);
        // Get node IDs.
        // Get node IDs.
        let node_id_1 = get_access_key_tree_node_id_test(access_key_1_2);
        let node_id_2 = get_access_key_list_node_id_test(access_key_1_2);
        // Remove sole entry, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_2) == 2, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_1, 0);
        assert!(get_list_top_test(&avlq) == node_id_2, 0);
        assert!(get_head_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_head_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_root_test(&avlq) == (NIL as u64), 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_1_ref = borrow_tree_node_test(&avlq, node_id_1);
        assert!(node_1_ref.bits == (NIL as u128), 0);
        // Assert inactive list node state indicates stack bottom.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
        // Init descending AVL queue.
        let avlq = new<u8>(DESCENDING, 0, 0);
        insert(&mut avlq, 1, 2); // Insert sole entry.
        // Remove sole entry, asserting returned insertion value.
        assert!(remove(&mut avlq, access_key_1_2) == 2, 0);
        // Assert AVL queue state.
        assert!(get_tree_top_test(&avlq) == node_id_1, 0);
        assert!(get_list_top_test(&avlq) == node_id_2, 0);
        assert!(get_head_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_head_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_node_id_test(&avlq) == (NIL as u64), 0);
        assert!(get_tail_key_test(&avlq) == (NIL as u64), 0);
        assert!(get_root_test(&avlq) == (NIL as u64), 0);
        // Assert inactive tree node state indicates stack bottom.
        let node_1_ref = borrow_tree_node_test(&avlq, node_id_1);
        assert!(node_1_ref.bits == (NIL as u128), 0);
        // Assert inactive list node state indicates stack bottom.
        assert!(!is_tree_node_list_last_by_id_test(&avlq, node_id_2), 0);
        assert!(get_list_last_node_id_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(!is_tree_node_list_next_by_id_test(&avlq, node_id_2), 0);
        assert!(get_list_next_node_id_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for tree having 1 at root and 2 as right
    /// child, removing the root twice.
    fun test_remove_root_twice() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        let (node_id_1, node_id_2) = // Insert nodes.
            (get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)));
        remove_tree_node(&mut avlq, node_id_1); // Remove root 1.
        assert!(get_root_test(&avlq) == node_id_2, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_id_1, 0);
        // Assert node 1 state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_1_ref = borrow_tree_node_test(&avlq, node_id_1);
        assert!(node_1_ref.bits == (NIL as u128), 0);
        // Assert node 2 state.
        assert!(get_insertion_key_by_id_test(&avlq, node_id_2) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_id_2) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_id_2)
            == (NIL as u64), 0);
        remove_tree_node(&mut avlq, node_id_2); // Remove root 2.
        // Assert root for empty tree.
        assert!(get_root_test(&avlq) == (NIL as u64), 0);
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_id_2, 0);
        // Assert node 2 state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_2_ref = borrow_tree_node_test(&avlq, node_id_2);
        assert!(node_2_ref.bits == 1, 0); // Node 1 allocated first.
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for reference operations in `retrace()`.
    fun test_retrace_insert_remove() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node IDs.
        let node_a_id = HI_NODE_ID;
        let node_b_id = node_a_id - 1;
        let node_c_id = node_b_id - 1;
        let node_d_id = node_c_id - 1;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram, with heights
        // not yet updated via insertion retrace.
        table_with_length::add(tree_nodes_ref_mut, node_a_id, TreeNode {
            bits:
            (3 as u128) << SHIFT_INSERTION_KEY |
                (node_b_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, node_b_id, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_a_id as u128) << SHIFT_CHILD_LEFT |
                (node_c_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_c_id, TreeNode {
            bits:
            (5 as u128) << SHIFT_INSERTION_KEY |
                (node_b_id as u128) << SHIFT_PARENT |
                (node_d_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_d_id, TreeNode {
            bits:
            (6 as u128) << SHIFT_INSERTION_KEY |
                (node_c_id as u128) << SHIFT_PARENT
        });
        // Set root node ID.
        set_root_test(&mut avlq, node_b_id);
        // Retrace from node c.
        retrace(&mut avlq, node_c_id, INCREMENT, RIGHT);
        // Assert state for node a.
        assert!(get_insertion_key_by_id_test(&avlq, node_a_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_a_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_a_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_a_id) == node_b_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_a_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_a_id)
            == (NIL as u64), 0);
        // Assert state for node b.
        assert!(get_insertion_key_by_id_test(&avlq, node_b_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_b_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_b_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_b_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_b_id) == node_a_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_b_id) == node_c_id, 0);
        // Assert state for node c.
        assert!(get_insertion_key_by_id_test(&avlq, node_c_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, node_c_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_c_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_c_id) == node_b_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_c_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_c_id) == node_d_id, 0);
        // Assert state for node d.
        assert!(get_insertion_key_by_id_test(&avlq, node_d_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_d_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_d_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_d_id) == node_c_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_d_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_d_id)
            == (NIL as u64), 0);
        // Assert root.
        assert!(get_root_test(&avlq) == node_b_id, 0);
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Simulate removing node d by clearing out node c's right child
        // field: remove and unpack node, then add new one with
        // corresponding state.
        let TreeNode { bits: _ } =
            table_with_length::remove(tree_nodes_ref_mut, node_c_id);
        table_with_length::add(tree_nodes_ref_mut, node_c_id, TreeNode {
            bits:
            (5 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_b_id as u128) << SHIFT_PARENT
        });
        // Retrace from node c.
        retrace(&mut avlq, node_c_id, DECREMENT, RIGHT);
        // Assert state for node a.
        assert!(get_insertion_key_by_id_test(&avlq, node_a_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_a_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_a_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_a_id) == node_b_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_a_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_a_id)
            == (NIL as u64), 0);
        // Assert state for node b.
        assert!(get_insertion_key_by_id_test(&avlq, node_b_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_b_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_b_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_b_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_b_id) == node_a_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_b_id) == node_c_id, 0);
        // Assert state for node c.
        assert!(get_insertion_key_by_id_test(&avlq, node_c_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, node_c_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_c_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_c_id) == node_b_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_c_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_c_id)
            == (NIL as u64), 0);
        // Assert root.
        assert!(get_root_test(&avlq) == node_b_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates/returns for `retrace_prep_iterate()` case
    /// 1.
    fun test_retrace_prep_iterate_1() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare arguments.
        let insertion_key = HI_INSERTION_KEY;
        let parent_id = HI_NODE_ID;
        let node_id = parent_id - 1;
        let new_subtree_root = node_id - 1;
        let sibling_id = new_subtree_root - 1;
        let height = 2;
        let height_old = 3;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert parent node.
        table_with_length::add(tree_nodes_ref_mut, parent_id, TreeNode {
            bits:
            (insertion_key as u128) << SHIFT_INSERTION_KEY |
                (node_id as u128) << SHIFT_CHILD_LEFT |
                (sibling_id as u128) << SHIFT_CHILD_RIGHT
        });
        // Prepare for next iteration, storing returns.
        let (node_ref_mut, operation, side, delta) = retrace_prep_iterate(
            &mut avlq, parent_id, node_id, new_subtree_root, height,
            height_old);
        // Assert insertion key accessed by mutable reference return.
        assert!(get_insertion_key_test(node_ref_mut) == insertion_key, 0);
        // Assert other returns.
        assert!(operation == DECREMENT, 0);
        assert!(side == LEFT, 0);
        assert!(delta == 1, 0);
        // Assert child fields of parent.
        assert!(get_child_left_by_id_test(&avlq, parent_id)
            == new_subtree_root, 0);
        assert!(get_child_right_by_id_test(&avlq, parent_id) == sibling_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates/returns for `retrace_prep_iterate()` case
    /// 2.
    fun test_retrace_prep_iterate_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare arguments.
        let insertion_key = HI_INSERTION_KEY;
        let parent_id = HI_NODE_ID;
        let node_id = parent_id - 1;
        let new_subtree_root = node_id - 1;
        let sibling_id = new_subtree_root - 1;
        let height = 3;
        let height_old = 3;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert parent node.
        table_with_length::add(tree_nodes_ref_mut, parent_id, TreeNode {
            bits:
            (insertion_key as u128) << SHIFT_INSERTION_KEY |
                (sibling_id as u128) << SHIFT_CHILD_LEFT |
                (node_id as u128) << SHIFT_CHILD_RIGHT
        });
        // Prepare for next iteration, storing returns.
        let (node_ref_mut, operation, side, delta) = retrace_prep_iterate(
            &mut avlq, parent_id, node_id, new_subtree_root, height,
            height_old);
        // Assert insertion key accessed by mutable reference return.
        assert!(get_insertion_key_test(node_ref_mut) == insertion_key, 0);
        // Assert other returns.
        assert!(operation == DECREMENT, 0);
        assert!(side == RIGHT, 0);
        assert!(delta == 0, 0);
        // Assert child fields of parent.
        assert!(get_child_left_by_id_test(&avlq, parent_id) == sibling_id, 0);
        assert!(get_child_right_by_id_test(&avlq, parent_id)
            == new_subtree_root, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates/returns for `retrace_prep_iterate()` case
    /// 3.
    fun test_retrace_prep_iterate_3() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare arguments.
        let insertion_key = HI_INSERTION_KEY;
        let parent_id = HI_NODE_ID;
        let node_id = parent_id - 1;
        let new_subtree_root = (NIL as u64);
        let height = 1;
        let height_old = 0;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert parent node.
        table_with_length::add(tree_nodes_ref_mut, parent_id, TreeNode {
            bits:
            (insertion_key as u128) << SHIFT_INSERTION_KEY |
                (node_id as u128) << SHIFT_CHILD_RIGHT
        });
        // Prepare for next iteration, storing returns.
        let (node_ref_mut, operation, side, delta) = retrace_prep_iterate(
            &mut avlq, parent_id, node_id, new_subtree_root, height,
            height_old);
        // Assert insertion key accessed by mutable reference return.
        assert!(get_insertion_key_test(node_ref_mut) == insertion_key, 0);
        // Assert other returns.
        assert!(operation == INCREMENT, 0);
        assert!(side == RIGHT, 0);
        assert!(delta == 1, 0);
        // Assert child fields of parent.
        assert!(get_child_left_by_id_test(&avlq, parent_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, parent_id) == node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates/returns for `retrace_update_heights()`
    /// case 1.
    fun test_retrace_update_heights_1() {
        // Declare arguments.
        let tree_node = TreeNode {
            bits:
            (3 as u128) << SHIFT_HEIGHT_LEFT |
                (1 as u128) << SHIFT_HEIGHT_RIGHT
        };
        let side = LEFT;
        let operation = DECREMENT;
        let delta = 1;
        // Update heights, storing returns.
        let (height_left, height_right, height, height_old) =
            retrace_update_heights(&mut tree_node, side, operation, delta);
        // Assert returns.
        assert!(height_left == 2, 0);
        assert!(height_right == 1, 0);
        assert!(height == 2, 0);
        assert!(height_old == 3, 0);
        // Assert node state.
        assert!(get_height_left_test(&tree_node) == 2, 0);
        assert!(get_height_right_test(&tree_node) == 1, 0);
        // Unpack tree node, dropping bits.
        let TreeNode { bits: _ } = tree_node;
    }

    #[test]
    /// Verify state updates/returns for `retrace_update_heights()`
    /// case 2.
    fun test_retrace_update_heights_2() {
        // Declare arguments.
        let tree_node = TreeNode {
            bits:
            (3 as u128) << SHIFT_HEIGHT_LEFT |
                (4 as u128) << SHIFT_HEIGHT_RIGHT
        };
        let side = RIGHT;
        let operation = INCREMENT;
        let delta = 1;
        // Update heights, storing returns.
        let (height_left, height_right, height, height_old) =
            retrace_update_heights(&mut tree_node, side, operation, delta);
        // Assert returns.
        assert!(height_left == 3, 0);
        assert!(height_right == 5, 0);
        assert!(height == 5, 0);
        assert!(height_old == 4, 0);
        // Assert node state.
        assert!(get_height_left_test(&tree_node) == 3, 0);
        assert!(get_height_right_test(&tree_node) == 5, 0);
        // Unpack tree node, dropping bits.
        let TreeNode { bits: _ } = tree_node;
    }

    #[test]
    /// Verify returns/state updates for
    /// `retrace_rebalance_rotate_left()` reference rotation 1.
    fun test_rotate_left_1() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node/tree IDs.
        let node_x_id = HI_NODE_ID;
        let node_z_id = node_x_id - 1;
        let tree_3_id = node_z_id - 1;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram.
        table_with_length::add(tree_nodes_ref_mut, node_x_id, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (2 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_z_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_z_id, TreeNode {
            bits:
            (6 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_x_id as u128) << SHIFT_PARENT |
                (tree_3_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_3_id, TreeNode {
            bits:
            (8 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        // Rebalance via left rotation, storing new subtree root node ID
        // and height.
        let (node_z_id_return, node_z_height_return) =
            retrace_rebalance(&mut avlq, node_x_id, node_z_id, false);
        // Assert returns.
        assert!(node_z_id_return == node_z_id, 0);
        assert!(node_z_height_return == 1, 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == node_x_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == tree_3_id, 0);
        // Assert state for tree 3.
        assert!(get_insertion_key_by_id_test(&avlq, tree_3_id) == 8, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_3_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `retrace_rebalance_rotate_left()`
    /// reference rotation 2.
    fun test_rotate_left_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert nodes from top to bottom, left to right.
        let (node_a_id, node_b_id, node_x_id, node_c_id, node_d_id, node_z_id,
            tree_2_id, tree_3_id) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 7, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 6, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 8, 0)));
        // Remove node d, rebalancing via left rotation.
        remove_tree_node(&mut avlq, node_d_id);
        assert!(get_root_test(&avlq) == node_a_id, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_d_id, 0);
        // Assert node d state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_d_ref = borrow_tree_node_test(&avlq, node_d_id);
        assert!(node_d_ref.bits == (NIL as u128), 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id) == tree_2_id, 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == node_a_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == node_x_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == tree_3_id, 0);
        // Assert state for tree 3.
        assert!(get_insertion_key_by_id_test(&avlq, tree_3_id) == 8, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_3_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        // Assert state for tree 2.
        assert!(get_insertion_key_by_id_test(&avlq, tree_2_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_2_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        // Assert state for node a.
        assert!(get_insertion_key_by_id_test(&avlq, node_a_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_a_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_a_id) == 3, 0);
        assert!(get_parent_by_id_test(&avlq, node_a_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_a_id) == node_b_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_a_id) == node_z_id, 0);
        // Assert state for node b.
        assert!(get_insertion_key_by_id_test(&avlq, node_b_id) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_b_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_b_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_b_id) == node_a_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_b_id) == node_c_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_b_id)
            == (NIL as u64), 0);
        // Assert state for node c.
        assert!(get_insertion_key_by_id_test(&avlq, node_c_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, node_c_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_c_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_c_id) == node_b_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_c_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_c_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `retrace_rebalance_rotate_left_right()`
    /// reference rotation 1.
    fun test_rotate_left_right_1() {
        // Declare number of allocated tree nodes.
        let n_allocated_tree_nodes = 12;
        // Init AVL queue.
        let avlq = new<u8>(ASCENDING, n_allocated_tree_nodes, 0);
        // Insert nodes from top to bottom, left to right.
        let (node_x_id, node_z_id, node_r_id, tree_1_id, node_y_id, tree_4_id,
            tree_3_id) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 6, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 7, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)));
        // Declare number of active tree nodes.
        let n_active_tree_nodes = 7;
        // Remove node r, rebalancing via left-right rotation.
        remove_tree_node(&mut avlq, node_r_id);
        assert!(get_root_test(&avlq) == node_y_id, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_r_id, 0);
        // Assert node r state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_r_ref = borrow_tree_node_test(&avlq, node_r_id);
        assert!(node_r_ref.bits ==
            ((n_allocated_tree_nodes - n_active_tree_nodes) as u128), 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id) == tree_3_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id) == tree_4_id, 0);
        // Assert state for node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_y_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_y_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_y_id) == node_z_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_y_id) == node_x_id, 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id)
            == (NIL as u64), 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert state for tree 3.
        assert!(get_insertion_key_by_id_test(&avlq, tree_3_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_3_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        // Assert state for tree 4.
        assert!(get_insertion_key_by_id_test(&avlq, tree_4_id) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_4_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns/state updates for
    /// `retrace_rebalance_rotate_left_right()` reference rotation 2.
    fun test_rotate_left_right_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node/tree IDs.
        let node_x_id = HI_NODE_ID;
        let node_z_id = node_x_id - 1;
        let node_y_id = node_z_id - 1;
        let tree_1_id = node_y_id - 1;
        let tree_2_id = tree_1_id - 1;
        let tree_4_id = tree_2_id - 1;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram.
        table_with_length::add(tree_nodes_ref_mut, node_x_id, TreeNode {
            bits:
            (8 as u128) << SHIFT_INSERTION_KEY |
                (3 as u128) << SHIFT_HEIGHT_LEFT |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_z_id as u128) << SHIFT_CHILD_LEFT |
                (tree_4_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_z_id, TreeNode {
            bits:
            (2 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (2 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_x_id as u128) << SHIFT_PARENT |
                (tree_1_id as u128) << SHIFT_CHILD_LEFT |
                (node_y_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_y_id, TreeNode {
            bits:
            (6 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (node_z_id as u128) << SHIFT_PARENT |
                (tree_2_id as u128) << SHIFT_CHILD_LEFT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_1_id, TreeNode {
            bits:
            (1 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_2_id, TreeNode {
            bits:
            (5 as u128) << SHIFT_INSERTION_KEY |
                (node_y_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_4_id, TreeNode {
            bits:
            (9 as u128) << SHIFT_INSERTION_KEY |
                (node_x_id as u128) << SHIFT_PARENT
        });
        // Rebalance via left-right rotation, storing new subtree root
        // node ID and height.
        let (node_y_id_return, node_y_height_return) =
            retrace_rebalance(&mut avlq, node_x_id, node_z_id, true);
        // Assert returns.
        assert!(node_y_id_return == node_y_id, 0);
        assert!(node_y_height_return == 2, 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 8, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id) == tree_4_id, 0);
        // Assert state for node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_y_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_y_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_y_id) == node_z_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_y_id) == node_x_id, 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == tree_2_id, 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert state for tree 2.
        assert!(get_insertion_key_by_id_test(&avlq, tree_2_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_2_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        // Assert state for tree 4.
        assert!(get_insertion_key_by_id_test(&avlq, tree_4_id) == 9, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_4_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns/state updates for
    /// `retrace_rebalance_rotate_right()` reference rotation 1.
    fun test_rotate_right_1() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node/tree IDs.
        let node_x_id = HI_NODE_ID;
        let node_z_id = node_x_id - 1;
        let tree_1_id = node_z_id - 1;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram, with heights
        // not yet updated via retrace.
        table_with_length::add(tree_nodes_ref_mut, node_x_id, TreeNode {
            bits:
            (8 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (node_z_id as u128) << SHIFT_CHILD_LEFT
        });
        table_with_length::add(tree_nodes_ref_mut, node_z_id, TreeNode {
            bits:
            (6 as u128) << SHIFT_INSERTION_KEY |
                (node_x_id as u128) << SHIFT_PARENT |
                (tree_1_id as u128) << SHIFT_CHILD_LEFT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_1_id, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        // Set root node ID.
        set_root_test(&mut avlq, node_z_id);
        // Retrace from node z, rebalancing via right rotation.
        retrace(&mut avlq, node_z_id, INCREMENT, LEFT);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 8, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == node_x_id, 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert root.
        assert!(get_root_test(&avlq) == node_z_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns/state updates for
    /// `retrace_rebalance_rotate_right()` reference rotation 2.
    fun test_rotate_right_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node/tree IDs.
        let node_x_id = HI_NODE_ID;
        let node_z_id = node_x_id - 1;
        let tree_1_id = node_z_id - 1;
        let tree_2_id = tree_1_id - 2;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram.
        table_with_length::add(tree_nodes_ref_mut, node_x_id, TreeNode {
            bits:
            (7 as u128) << SHIFT_INSERTION_KEY |
                (2 as u128) << SHIFT_HEIGHT_LEFT |
                (NIL as u128) << SHIFT_PARENT |
                (node_z_id as u128) << SHIFT_CHILD_LEFT
        });
        table_with_length::add(tree_nodes_ref_mut, node_z_id, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_x_id as u128) << SHIFT_PARENT |
                (tree_1_id as u128) << SHIFT_CHILD_LEFT |
                (tree_2_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_1_id, TreeNode {
            bits:
            (3 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_2_id, TreeNode {
            bits:
            (5 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        // Rebalance via right rotation, storing new subtree root node
        // ID and height.
        let (node_z_id_return, node_z_height_return) =
            retrace_rebalance(&mut avlq, node_x_id, node_z_id, true);
        // Assert returns.
        assert!(node_z_id_return == node_z_id, 0);
        assert!(node_z_height_return == 2, 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id) == tree_2_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == node_x_id, 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert state for tree 2.
        assert!(get_insertion_key_by_id_test(&avlq, tree_2_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_2_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns/state updates for
    /// `retrace_rebalance_rotate_right_left()` reference rotation 1.
    fun test_rotate_right_left_1() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Declare node/tree IDs.
        let node_x_id = HI_NODE_ID;
        let node_z_id = node_x_id - 1;
        let node_y_id = node_z_id - 1;
        let tree_1_id = node_y_id - 1;
        let tree_2_id = tree_1_id - 1;
        let tree_4_id = tree_2_id - 1;
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram.
        table_with_length::add(tree_nodes_ref_mut, node_x_id, TreeNode {
            bits:
            (2 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (3 as u128) << SHIFT_HEIGHT_RIGHT |
                (tree_1_id as u128) << SHIFT_CHILD_LEFT |
                (node_z_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_z_id, TreeNode {
            bits:
            (8 as u128) << SHIFT_INSERTION_KEY |
                (2 as u128) << SHIFT_HEIGHT_LEFT |
                (1 as u128) << SHIFT_HEIGHT_RIGHT |
                (node_x_id as u128) << SHIFT_PARENT |
                (node_y_id as u128) << SHIFT_CHILD_LEFT |
                (tree_4_id as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, node_y_id, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_HEIGHT_LEFT |
                (node_z_id as u128) << SHIFT_PARENT |
                (tree_2_id as u128) << SHIFT_CHILD_LEFT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_1_id, TreeNode {
            bits:
            (1 as u128) << SHIFT_INSERTION_KEY |
                (node_x_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_2_id, TreeNode {
            bits:
            (3 as u128) << SHIFT_INSERTION_KEY |
                (node_y_id as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, tree_4_id, TreeNode {
            bits:
            (9 as u128) << SHIFT_INSERTION_KEY |
                (node_z_id as u128) << SHIFT_PARENT
        });
        // Rebalance via right-left rotation, storing new subtree root
        // node ID and height.
        let (node_y_id_return, node_y_height_return) =
            retrace_rebalance(&mut avlq, node_x_id, node_z_id, false);
        // Assert returns.
        assert!(node_y_id_return == node_y_id, 0);
        assert!(node_y_height_return == 2, 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 2, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id) == tree_2_id, 0);
        // Assert state for node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_y_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_y_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_y_id) == node_x_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_y_id) == node_z_id, 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 8, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == tree_4_id, 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert state for tree 2.
        assert!(get_insertion_key_by_id_test(&avlq, tree_2_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_2_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_2_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_2_id)
            == (NIL as u64), 0);
        // Assert state for tree 4.
        assert!(get_insertion_key_by_id_test(&avlq, tree_4_id) == 9, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_4_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify state updates for `retrace_rebalance_rotate_right_left()`
    /// reference rotation 2.
    fun test_rotate_right_left_2() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert nodes from top to bottom, left to right.
        let (node_x_id, node_r_id, node_z_id, tree_1_id, node_y_id, tree_4_id,
            tree_3_id) =
            (get_access_key_tree_node_id_test(insert(&mut avlq, 3, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 2, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 6, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 1, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 4, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 7, 0)),
                get_access_key_tree_node_id_test(insert(&mut avlq, 5, 0)));
        // Remove node r, rebalancing via right-left rotation.
        remove_tree_node(&mut avlq, node_r_id);
        assert!(get_root_test(&avlq) == node_y_id, 0); // Assert root.
        // Assert inactive tree nodes stack top.
        assert!(get_tree_top_test(&avlq) == node_r_id, 0);
        // Assert node r state contains only bits for node ID of next
        // tree node ID in inactive tree node stack.
        let node_r_ref = borrow_tree_node_test(&avlq, node_r_id);
        assert!(node_r_ref.bits == (NIL as u128), 0);
        // Assert state for node x.
        assert!(get_insertion_key_by_id_test(&avlq, node_x_id) == 3, 0);
        assert!(get_height_left_by_id_test(&avlq, node_x_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_x_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, node_x_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_x_id) == tree_1_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_x_id)
            == (NIL as u64), 0);
        // Assert state for node y.
        assert!(get_insertion_key_by_id_test(&avlq, node_y_id) == 4, 0);
        assert!(get_height_left_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_height_right_by_id_test(&avlq, node_y_id) == 2, 0);
        assert!(get_parent_by_id_test(&avlq, node_y_id) == (NIL as u64), 0);
        assert!(get_child_left_by_id_test(&avlq, node_y_id) == node_x_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_y_id) == node_z_id, 0);
        // Assert state for node z.
        assert!(get_insertion_key_by_id_test(&avlq, node_z_id) == 6, 0);
        assert!(get_height_left_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_height_right_by_id_test(&avlq, node_z_id) == 1, 0);
        assert!(get_parent_by_id_test(&avlq, node_z_id) == node_y_id, 0);
        assert!(get_child_left_by_id_test(&avlq, node_z_id) == tree_3_id, 0);
        assert!(get_child_right_by_id_test(&avlq, node_z_id) == tree_4_id, 0);
        // Assert state for tree 1.
        assert!(get_insertion_key_by_id_test(&avlq, tree_1_id) == 1, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_1_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_1_id) == node_x_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_1_id)
            == (NIL as u64), 0);
        // Assert state for tree 3.
        assert!(get_insertion_key_by_id_test(&avlq, tree_3_id) == 5, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_3_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_3_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_3_id)
            == (NIL as u64), 0);
        // Assert state for tree 4.
        assert!(get_insertion_key_by_id_test(&avlq, tree_4_id) == 7, 0);
        assert!(get_height_left_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_height_right_by_id_test(&avlq, tree_4_id) == 0, 0);
        assert!(get_parent_by_id_test(&avlq, tree_4_id) == node_z_id, 0);
        assert!(get_child_left_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        assert!(get_child_right_by_id_test(&avlq, tree_4_id)
            == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify returns for reference diagram in `search()`.
    fun test_search() {
        // Init ascending AVL queue.
        let avlq = new<u8>(ASCENDING, 0, 0);
        // Assert returns for when empty.
        let (node_id, side_option) = search(&mut avlq, 12345);
        assert!(node_id == (NIL as u64), 0);
        assert!(option::is_none(&side_option), 0);
        // Manually set root.
        set_root_test(&mut avlq, 1);
        // Mutably borrow tree nodes table.
        let tree_nodes_ref_mut = &mut avlq.tree_nodes;
        // Manually insert nodes from reference diagram.
        table_with_length::add(tree_nodes_ref_mut, 1, TreeNode {
            bits:
            (4 as u128) << SHIFT_INSERTION_KEY |
                (5 as u128) << SHIFT_CHILD_LEFT |
                (2 as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, 2, TreeNode {
            bits:
            (8 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_PARENT |
                (4 as u128) << SHIFT_CHILD_LEFT |
                (3 as u128) << SHIFT_CHILD_RIGHT
        });
        table_with_length::add(tree_nodes_ref_mut, 3, TreeNode {
            bits:
            (10 as u128) << SHIFT_INSERTION_KEY |
                (2 as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, 4, TreeNode {
            bits:
            (6 as u128) << SHIFT_INSERTION_KEY |
                (2 as u128) << SHIFT_PARENT
        });
        table_with_length::add(tree_nodes_ref_mut, 5, TreeNode {
            bits:
            (2 as u128) << SHIFT_INSERTION_KEY |
                (1 as u128) << SHIFT_PARENT
        });
        // Assert returns in order from reference table.
        (node_id, side_option) = search(&mut avlq, 2);
        let node_ref = borrow_tree_node_test(&avlq, node_id);
        assert!(get_insertion_key_test(node_ref) == 2, 0);
        assert!(node_id == 5, 0);
        assert!(option::is_none(&side_option), 0);
        (node_id, side_option) = search(&mut avlq, 7);
        node_ref = borrow_tree_node_test(&avlq, node_id);
        assert!(get_insertion_key_test(node_ref) == 6, 0);
        assert!(node_id == 4, 0);
        assert!(*option::borrow(&side_option) == RIGHT, 0);
        (node_id, side_option) = search(&mut avlq, 9);
        node_ref = borrow_tree_node_test(&avlq, node_id);
        assert!(get_insertion_key_test(node_ref) == 10, 0);
        assert!(node_id == 3, 0);
        assert!(*option::borrow(&side_option) == LEFT, 0);
        (node_id, side_option) = search(&mut avlq, 4);
        node_ref = borrow_tree_node_test(&avlq, node_id);
        assert!(get_insertion_key_test(node_ref) == 4, 0);
        assert!(node_id == 1, 0);
        assert!(option::is_none(&side_option), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful state operations.
    fun test_set_get_head_tail_test() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        avlq.bits = 0; // Clear out all bits.
        // Declare head and tail keys, node IDs.
        let head_key = u_64(b"10000000000000000000000000000001");
        let tail_key = u_64(b"11000000000000000000000000000011");
        let head_node_id = u_64(b"10000000000001");
        let tail_node_id = u_64(b"11000000000011");
        // Set head and tail keys, node IDs.
        set_head_key_test(&mut avlq, head_key);
        set_tail_key_test(&mut avlq, tail_key);
        set_head_node_id_test(&mut avlq, head_node_id);
        set_tail_node_id_test(&mut avlq, tail_node_id);
        // Assert bit fields.
        assert!(avlq.bits == u_128_by_32(
            b"00000000000000000000000000000010",
            //                              ^ bit 97
            b"00000000000110000000000000000000",
            //    bit 84 ^^ bit 83
            b"00000000000111000000000011110000",
            //    bit 52 ^^ bits 38-51 ^^ bit 37
            b"00000000000000000000000011000000"), 0);
        //                         ^ bit 6
        // Assert getter returns.
        assert!(get_head_key_test(&avlq) == head_key, 0);
        assert!(get_tail_key_test(&avlq) == tail_key, 0);
        assert!(get_head_node_id_test(&avlq) == head_node_id, 0);
        assert!(get_tail_node_id_test(&avlq) == tail_node_id, 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful state operations.
    fun test_set_get_root_test() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        avlq.bits = u_128_by_32(// Set all bits.
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111");
        avlq.root_lsbs = (u_64(b"11111111") as u8); // Set all bits.
        // Assert getter return.
        assert!(get_root_test(&avlq) == HI_NODE_ID, 0);
        let new_root = u_64(b"10000000000001"); // Declare new root.
        set_root_test(&mut avlq, new_root); // Set new root.
        // Assert getter return.
        assert!(get_root_test(&avlq) == new_root, 0);
        // Assert fields.
        assert!(avlq.bits == u_128_by_32(
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111100000"), 0);
        assert!(avlq.root_lsbs == (u_64(b"00000001") as u8), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful returns for reference diagram in `traverse()`,
    /// with two list nodes in each tree node.
    fun test_traverse() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Insert root from reference diagram.
        let access_key_5_1 = (insert(&mut avlq, 5, 1));
        let access_key_5_2 = (insert(&mut avlq, 5, 2));
        let tree_id_5 = get_access_key_tree_node_id_test(access_key_5_1);
        let list_id_5_1 = get_access_key_list_node_id_test(access_key_5_1);
        let list_id_5_2 = get_access_key_list_node_id_test(access_key_5_2);
        // Assert returns for traversal at root.
        let (key, head, tail) = traverse(&avlq, tree_id_5, PREDECESSOR);
        assert!(key == (NIL as u64), 0);
        assert!(head == (NIL as u64), 0);
        assert!(tail == (NIL as u64), 0);
        (key, head, tail) = traverse(&avlq, tree_id_5, SUCCESSOR);
        assert!(key == (NIL as u64), 0);
        assert!(head == (NIL as u64), 0);
        assert!(tail == (NIL as u64), 0);
        // Insert in remaining sequence from reference diagram.
        let access_key_8_1 = insert(&mut avlq, 8, 1);
        let access_key_8_2 = insert(&mut avlq, 8, 2);
        let tree_id_8 = get_access_key_tree_node_id_test(access_key_8_1);
        let list_id_8_1 = get_access_key_list_node_id_test(access_key_8_1);
        let list_id_8_2 = get_access_key_list_node_id_test(access_key_8_2);
        let access_key_2_1 = insert(&mut avlq, 2, 1);
        let access_key_2_2 = insert(&mut avlq, 2, 2);
        let tree_id_2 = get_access_key_tree_node_id_test(access_key_2_1);
        let list_id_2_1 = get_access_key_list_node_id_test(access_key_2_1);
        let list_id_2_2 = get_access_key_list_node_id_test(access_key_2_2);
        let access_key_1_1 = insert(&mut avlq, 1, 1);
        let access_key_1_2 = insert(&mut avlq, 1, 2);
        let tree_id_1 = get_access_key_tree_node_id_test(access_key_1_1);
        let list_id_1_1 = get_access_key_list_node_id_test(access_key_1_1);
        let list_id_1_2 = get_access_key_list_node_id_test(access_key_1_2);
        let access_key_3_1 = insert(&mut avlq, 3, 1);
        let access_key_3_2 = insert(&mut avlq, 3, 2);
        let tree_id_3 = get_access_key_tree_node_id_test(access_key_3_1);
        let list_id_3_1 = get_access_key_list_node_id_test(access_key_3_1);
        let list_id_3_2 = get_access_key_list_node_id_test(access_key_3_2);
        let access_key_7_1 = insert(&mut avlq, 7, 1);
        let access_key_7_2 = insert(&mut avlq, 7, 2);
        let tree_id_7 = get_access_key_tree_node_id_test(access_key_7_1);
        let list_id_7_1 = get_access_key_list_node_id_test(access_key_7_1);
        let list_id_7_2 = get_access_key_list_node_id_test(access_key_7_2);
        let access_key_9_1 = insert(&mut avlq, 9, 1);
        let access_key_9_2 = insert(&mut avlq, 9, 2);
        let tree_id_9 = get_access_key_tree_node_id_test(access_key_9_1);
        let list_id_9_1 = get_access_key_list_node_id_test(access_key_9_1);
        let list_id_9_2 = get_access_key_list_node_id_test(access_key_9_2);
        let access_key_4_1 = insert(&mut avlq, 4, 1);
        let access_key_4_2 = insert(&mut avlq, 4, 2);
        let tree_id_4 = get_access_key_tree_node_id_test(access_key_4_1);
        let list_id_4_1 = get_access_key_list_node_id_test(access_key_4_1);
        let list_id_4_2 = get_access_key_list_node_id_test(access_key_4_2);
        let access_key_6_1 = insert(&mut avlq, 6, 1);
        let access_key_6_2 = insert(&mut avlq, 6, 2);
        let tree_id_6 = get_access_key_tree_node_id_test(access_key_6_1);
        let list_id_6_1 = get_access_key_list_node_id_test(access_key_6_1);
        let list_id_6_2 = get_access_key_list_node_id_test(access_key_6_2);
        // Assert predecessor returns.
        (key, head, tail) = traverse(&avlq, tree_id_1, PREDECESSOR);
        assert!(key == (NIL as u64), 0);
        assert!(head == (NIL as u64), 0);
        assert!(tail == (NIL as u64), 0);
        (key, head, tail) = traverse(&avlq, tree_id_2, PREDECESSOR);
        assert!(key == 1, 0);
        assert!(head == list_id_1_1, 0);
        assert!(tail == list_id_1_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_3, PREDECESSOR);
        assert!(key == 2, 0);
        assert!(head == list_id_2_1, 0);
        assert!(tail == list_id_2_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_4, PREDECESSOR);
        assert!(key == 3, 0);
        assert!(head == list_id_3_1, 0);
        assert!(tail == list_id_3_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_5, PREDECESSOR);
        assert!(key == 4, 0);
        assert!(head == list_id_4_1, 0);
        assert!(tail == list_id_4_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_6, PREDECESSOR);
        assert!(key == 5, 0);
        assert!(head == list_id_5_1, 0);
        assert!(tail == list_id_5_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_7, PREDECESSOR);
        assert!(key == 6, 0);
        assert!(head == list_id_6_1, 0);
        assert!(tail == list_id_6_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_8, PREDECESSOR);
        assert!(key == 7, 0);
        assert!(head == list_id_7_1, 0);
        assert!(tail == list_id_7_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_9, PREDECESSOR);
        assert!(key == 8, 0);
        assert!(head == list_id_8_1, 0);
        assert!(tail == list_id_8_2, 0);
        // Assert successor returns.
        (key, head, tail) = traverse(&avlq, tree_id_1, SUCCESSOR);
        assert!(key == 2, 0);
        assert!(head == list_id_2_1, 0);
        assert!(tail == list_id_2_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_2, SUCCESSOR);
        assert!(key == 3, 0);
        assert!(head == list_id_3_1, 0);
        assert!(tail == list_id_3_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_3, SUCCESSOR);
        assert!(key == 4, 0);
        assert!(head == list_id_4_1, 0);
        assert!(tail == list_id_4_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_4, SUCCESSOR);
        assert!(key == 5, 0);
        assert!(head == list_id_5_1, 0);
        assert!(tail == list_id_5_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_5, SUCCESSOR);
        assert!(key == 6, 0);
        assert!(head == list_id_6_1, 0);
        assert!(tail == list_id_6_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_6, SUCCESSOR);
        assert!(key == 7, 0);
        assert!(head == list_id_7_1, 0);
        assert!(tail == list_id_7_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_7, SUCCESSOR);
        assert!(key == 8, 0);
        assert!(head == list_id_8_1, 0);
        assert!(tail == list_id_8_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_8, SUCCESSOR);
        assert!(key == 9, 0);
        assert!(head == list_id_9_1, 0);
        assert!(tail == list_id_9_2, 0);
        (key, head, tail) = traverse(&avlq, tree_id_9, SUCCESSOR);
        assert!(key == (NIL as u64), 0);
        assert!(head == (NIL as u64), 0);
        assert!(tail == (NIL as u64), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    /// Verify successful return values.
    fun test_u_128_64() {
        assert!(u_128(b"0") == 0, 0);
        assert!(u_128(b"1") == 1, 0);
        assert!(u_128(b"00") == 0, 0);
        assert!(u_128(b"01") == 1, 0);
        assert!(u_128(b"10") == 2, 0);
        assert!(u_128(b"11") == 3, 0);
        assert!(u_128(b"10101010") == 170, 0);
        assert!(u_128(b"00000001") == 1, 0);
        assert!(u_128(b"11111111") == 255, 0);
        assert!(u_128_by_32(
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111"
        ) == HI_128, 0);
        assert!(u_128_by_32(
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111110"
        ) == HI_128 - 1, 0);
        assert!(u_64(b"0") == 0, 0);
        assert!(u_64(b"0") == 0, 0);
        assert!(u_64(b"1") == 1, 0);
        assert!(u_64(b"00") == 0, 0);
        assert!(u_64(b"01") == 1, 0);
        assert!(u_64(b"10") == 2, 0);
        assert!(u_64(b"11") == 3, 0);
        assert!(u_64(b"10101010") == 170, 0);
        assert!(u_64(b"00000001") == 1, 0);
        assert!(u_64(b"11111111") == 255, 0);
        assert!(u_64_by_32(
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111111"
        ) == HI_64, 0);
        assert!(u_64_by_32(
            b"11111111111111111111111111111111",
            b"11111111111111111111111111111110"
        ) == HI_64 - 1, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_BIT_NOT_0_OR_1)]
    /// Verify failure for non-binary-representative byte string.
    fun test_u_128_failure() { u_128(b"2"); }

    #[test]
    /// Verify expected returns.
    fun test_would_update_head_tail() {
        // Init ascending AVL queue.
        let avlq = new<u8>(ASCENDING, 0, 0);
        // Assert returns for empty AVL queue.
        assert!(would_update_head(&avlq, 0), 0);
        assert!(would_update_tail(&avlq, 0), 0);
        assert!(would_update_head(&avlq, HI_INSERTION_KEY), 0);
        assert!(would_update_tail(&avlq, HI_INSERTION_KEY), 0);
        insert(&mut avlq, 1, 0); // Insert key 1 above min.
        // Assert returns 1 less than, equal to, and 1 greater than key.
        assert!(would_update_head(&avlq, 0), 0);
        assert!(!would_update_head(&avlq, 1), 0);
        assert!(!would_update_head(&avlq, 2), 0);
        assert!(!would_update_tail(&avlq, 0), 0);
        assert!(would_update_tail(&avlq, 1), 0);
        assert!(would_update_tail(&avlq, 2), 0);
        // Insert key 1 below max.
        insert(&mut avlq, HI_INSERTION_KEY - 1, 0);
        // Assert returns 1 less than, equal to, and 1 greater than key.
        assert!(!would_update_head(&avlq, HI_INSERTION_KEY - 2), 0);
        assert!(!would_update_head(&avlq, HI_INSERTION_KEY - 1), 0);
        assert!(!would_update_head(&avlq, HI_INSERTION_KEY), 0);
        assert!(!would_update_tail(&avlq, HI_INSERTION_KEY - 2), 0);
        assert!(would_update_tail(&avlq, HI_INSERTION_KEY - 1), 0);
        assert!(would_update_tail(&avlq, HI_INSERTION_KEY), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
        // Init descending AVL queue.
        let avlq = new<u8>(DESCENDING, 0, 0);
        // Assert returns for empty AVL queue.
        assert!(would_update_head(&avlq, 0), 0);
        assert!(would_update_tail(&avlq, 0), 0);
        assert!(would_update_head(&avlq, HI_INSERTION_KEY), 0);
        assert!(would_update_tail(&avlq, HI_INSERTION_KEY), 0);
        insert(&mut avlq, 1, 0); // Insert key 1 above min.
        // Assert returns 1 less than, equal to, and 1 greater than key.
        assert!(!would_update_head(&avlq, 0), 0);
        assert!(!would_update_head(&avlq, 1), 0);
        assert!(would_update_head(&avlq, 2), 0);
        assert!(would_update_tail(&avlq, 0), 0);
        assert!(would_update_tail(&avlq, 1), 0);
        assert!(!would_update_tail(&avlq, 2), 0);
        // Insert key 1 below max.
        insert(&mut avlq, HI_INSERTION_KEY - 1, 0);
        // Assert returns 1 less than, equal to, and 1 greater than key.
        assert!(!would_update_head(&avlq, HI_INSERTION_KEY - 2), 0);
        assert!(!would_update_head(&avlq, HI_INSERTION_KEY - 1), 0);
        assert!(would_update_head(&avlq, HI_INSERTION_KEY), 0);
        assert!(!would_update_tail(&avlq, HI_INSERTION_KEY - 2), 0);
        assert!(!would_update_tail(&avlq, HI_INSERTION_KEY - 1), 0);
        assert!(!would_update_tail(&avlq, HI_INSERTION_KEY), 0);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_INSERTION_KEY_TOO_LARGE)]
    /// Verify failure for insertion key too large.
    fun test_would_update_head_too_big() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Attempt invalid invocation.
        would_update_head(&avlq, HI_INSERTION_KEY + 1);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    #[test]
    #[expected_failure(abort_code = E_INSERTION_KEY_TOO_LARGE)]
    /// Verify failure for insertion key too large.
    fun test_would_update_tail_too_big() {
        let avlq = new<u8>(ASCENDING, 0, 0); // Init AVL queue.
        // Attempt invalid invocation.
        would_update_tail(&avlq, HI_INSERTION_KEY + 1);
        drop_avlq_test(avlq); // Drop AVL queue.
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
}

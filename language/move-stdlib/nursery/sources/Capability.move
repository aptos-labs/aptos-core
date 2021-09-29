/// A module which defines the basic concept of
/// [*capabilities*](https://en.wikipedia.org/wiki/Capability-based_security) for managing access control.
///
/// # Overview
///
/// A capability is a unforgeable token which testifies that a signer has authorized a certain operation.
/// The token is valid during the transaction where it is obtained. Since the type `Capability::Cap` has
/// no ability to be stored in global memory, capabilities cannot leak out of a transaction. For every function
/// called within a transaction which has a capability as a parameter, it is guaranteed that the capability
/// has been obtained via a proper signer-based authorization step previously in the transaction's execution.
///
/// ## Basic Usage
///
/// Capabilities are used typically as follows:
///
/// ```
///   struct ProtectedFeature { ... } // this can be just a type tag, or actually some protected data
///
///   public fun initialize(s: &signer) {
///     // Create capability. This happens once at module initialization time.
///     Capability::create<ProtectedFeature>(s);
///   }
///
///   public fun do_something(s: &signer) {
///     // Acquire the capability. This is the authorization step. Must have a signer to do so.
///     let cap = Capability::acquire<ProtectedFeature>(s);
///     // Pass the capability on to functions which require authorization.
///     critical(cap);
///   }
///
///   fun critical(cap: Capability::Cap<ProtectedFeature>) {
///     // Authorization guaranteed by construction -- no verification needed!
///     ...
///   }
/// ```
///
/// Notice that a key feature of capabilities is that they do not require extra verification steps
/// to ensure authorization is valid.
///
/// ## Advanced Authorization Scenarios
///
/// In the basic usage above, in order to acquire `Cap<ProtectedFeature>`, we needed a signer
/// that owns this capability. Because `Capability::acquires` is a public function, everybody can
/// acquire the capability provided the right signer is presented. But what if there authorization
/// scenarios which go beyond having a signer?
///
/// The current way how to achieve this in Move is to build a wrapper around `Capability::Cap<T>`.
/// The wrapper type will be owned by a specific module, restricting how values of it can be created.
/// Below, we extend the example from above to illustrate this pattern:
///
/// ```
/// struct ProtectedFeatureCap has copy, drop {
///     cap: Capability::Cap<ProtectedFeature>
/// }
///
/// public fun acquire_protected_feature_access(s: &signer): ProtectedFeatureCap {
///   let cap = Capability::acquire<ProtectedFeature>(s);
///   validate_authorization(s, cap); // Do any additional authorization validation
///   ProtectedFeatureCap{cap}
/// }
/// ```
///
/// ## Delegation
///
/// Capabilities come with the optional feature of *delegation*. Via delegation, an owner of a capability
/// can designate another signer to be also capable of acquiring the capability. Like the original owner,
/// the delegate needs to present his signer to obtain the capability in his transactions. Delegation can
/// be revoked, removing this access right from the delegate.
///
/// While the basic authorization mechanism for delegates is the same as with core capabilities, the
/// target of delegation might be subject of restrictions which need to be specified and verified. This can
/// be done via global invariants in the specification language. For example, in order to prevent delegation
/// all together for a capability, one can use the following invariant:
///
/// ```
///   invariant forall a: address where exists<CapState<ProtectedFeature>>(addr):
///               len(Capability::spec_delegates<ProtectedFeature>(a)) == 0;
/// ```
///
/// Similarly, the following invariant would enforce that delegates, if existent, must satisfy a certain
/// predicate:
///
/// ```
///   invariant forall a: address where exists<CapState<ProtectedFeature>>(addr):
///               forall d in Capability::spec_delegates<ProtectedFeature>(a):
///                  is_valid_delegate_for_protected_feature(d);
/// ```
///
module Std::Capability {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;

    const ECAP: u64 = 0;
    const EDELEGATE: u64 = 1;

    /// The token representing an acquired capability. Cannot be stored in memory, but copied and dropped freely.
    struct Cap<phantom Feature> has copy, drop {
        root: address
    }

    /// An internal data structure for representing a configured capability.
    struct CapState<phantom Feature> has key {
        delegates: vector<address>
    }

    /// An internal data structure for representing a configured delegated capability.
    struct CapDelegateState<phantom Feature> has key {
        root: address
    }

    /// Creates a new capability class, owned by the passed signer.
    public fun create<Feature>(owner: &signer) {
        let addr = Signer::address_of(owner);
        assert(!exists<CapState<Feature>>(addr), Errors::already_published(ECAP));
        move_to<CapState<Feature>>(owner, CapState{ delegates: Vector::empty() });
    }

    /// Acquires a capability token. Only the owner of the capability class, or an authorized delegate,
    /// can succeed with this operation.
    public fun acquire<Feature>(requester: &signer): Cap<Feature>
    acquires CapState, CapDelegateState {
        let addr = Signer::address_of(requester);
        if (exists<CapDelegateState<Feature>>(addr)) {
            let root_addr = borrow_global<CapDelegateState<Feature>>(addr).root;
            // double check that requester is actually registered as a delegate
            assert(exists<CapState<Feature>>(root_addr), Errors::invalid_state(EDELEGATE));
            assert(Vector::contains(&borrow_global<CapState<Feature>>(root_addr).delegates, &addr),
                   Errors::invalid_state(EDELEGATE));
            Cap<Feature>{root: root_addr}
        } else {
            assert(exists<CapState<Feature>>(addr), Errors::not_published(ECAP));
            Cap<Feature>{root: addr}
        }
    }

    /// Registers a delegation relation.
    public fun delegate<Feature>(cap: Cap<Feature>, to: &signer)
    acquires CapState {
        let addr = Signer::address_of(to);
        assert(!exists<CapDelegateState<Feature>>(addr), Errors::already_published(EDELEGATE));
        assert(exists<CapState<Feature>>(cap.root), Errors::invalid_state(ECAP));
        move_to(to, CapDelegateState<Feature>{root: cap.root});
        add_element(&mut borrow_global_mut<CapState<Feature>>(cap.root).delegates, addr);
    }

    /// Revokes a delegation relation.
    public fun revoke<Feature>(cap: Cap<Feature>, from: address)
    acquires CapState, CapDelegateState
    {
        assert(exists<CapDelegateState<Feature>>(from), Errors::not_published(EDELEGATE));
        assert(exists<CapState<Feature>>(cap.root), Errors::invalid_state(ECAP));
        let CapDelegateState{root: _root} = move_from<CapDelegateState<Feature>>(from);
        remove_element(&mut borrow_global_mut<CapState<Feature>>(cap.root).delegates, &from);
    }

    /// Helper to remove an element from a vector.
    fun remove_element<E: drop>(v: &mut vector<E>, x: &E) {
        let (found, index) = Vector::index_of(v, x);
        if (found) {
            Vector::remove(v, index);
        }
    }

    /// Helper to add an element to a vector.
    fun add_element<E: drop>(v: &mut vector<E>, x: E) {
        if (!Vector::contains(v, &x)) {
            Vector::push_back(v, x)
        }
    }

    /// Helper specification function to obtain the delegates of a capability.
    spec fun spec_delegates<Feature>(addr: address): vector<address> {
        global<CapState<Feature>>(addr).delegates
    }
}
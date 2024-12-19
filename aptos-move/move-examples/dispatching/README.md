This is an example for dynamic dispatch engine in Move.

* Publish the `dispatching` package.
* During publishing the initial state is setup.
* Register new callbacks by calling `storage::register<T: drop>(callback: FunctionInfo, _proof: T)`.
  * Registration is guarded by `T` instances, droppable instances that only the registering module can instantiate.
  * The callback must be of the form `public fun callback<T: key>(_metadata: Object<T>): option::Option<u128>`.
  * The callback can retrieve its data by calling `storage::retrieve<T: drop>(_proof: T): vector<u8>`.
* Dispatch work by calling `engine::dipatch<T>(data: vector<u8>)` either in Move or from a transaction payload.

spec aptos_framework::object {

    spec exists_at<T: key>(object: address): bool {
        pragma intrinsic;
    }

    spec address_to_object<T: key>(object: address): Object<T> {
        aborts_if !exists<ObjectCore>(object);
        aborts_if !exists<T>(object);
    }

    spec convert<X: key, Y: key>(object: Object<X>): Object<Y> {
        aborts_if !exists<ObjectCore>(object.inner);
        aborts_if !exists<Y>(object.inner);
    }

    spec object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !exists<T>(ref.self);
    }

    spec object_from_delete_ref<T: key>(ref: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !exists<T>(ref.self);
    }

}

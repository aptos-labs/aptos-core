#[macro_export]
macro_rules! gvk {
    ($kind:expr) => {{
        match $kind {
            k8s_openapi::api::core::v1::Pod::KIND => Some(kube::api::GroupVersionKind {
                group: k8s_openapi::api::core::v1::Pod::GROUP.to_string(),
                version: k8s_openapi::api::core::v1::Pod::VERSION.to_string(),
                kind: k8s_openapi::api::core::v1::Pod::KIND.to_string(),
            }),
            _ => None,
        }
        .unwrap()
    }};
}

#[macro_export]
macro_rules! kube_api {
    ($client:expr, $kind:expr) => {{
        let gvk = crate::gvk!($kind);
        let ar = kube::api::ApiResource::from_gvk(&gvk);
        kube::api::Api::<kube::api::DynamicObject>::all_with($client.clone(), &ar)
    }};
    ($client:expr, $kind:expr, $namespace:expr) => {{
        let gvk = crate::gvk!($kind);
        let ar = kube::api::ApiResource::from_gvk(&gvk);
        kube::api::Api::<kube::api::DynamicObject>::namespaced_with(
            $client.clone(),
            &$namespace,
            &ar,
        )
    }};
}

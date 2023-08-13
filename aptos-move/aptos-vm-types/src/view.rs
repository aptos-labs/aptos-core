use aptos_state_view::{StateViewId, TStateView as TRawStateView};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};

pub trait TResourceView {
    type Key;

    fn get_resource_bytes_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let maybe_state_value = self.get_resource_state_value_from_view(state_key)?;
        Ok(maybe_state_value.map(|state_value| state_value.into_bytes()))
    }

    fn get_resource_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>>;
}

impl<T: TRawStateView> TResourceView for T {
    type Key = T::Key;

    fn get_resource_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }
}

pub trait ResourceView: TResourceView<Key = StateKey> {}

pub trait TModuleView {
    type Key;

    fn get_module_bytes_from_view(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        let maybe_state_value = self.get_module_state_value_from_view(state_key)?;
        Ok(maybe_state_value.map(|state_value| state_value.into_bytes()))
    }

    fn get_module_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>>;
}

impl<T: TRawStateView> TModuleView for T {
    type Key = T::Key;

    fn get_module_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }
}

pub trait ModuleView: TModuleView<Key = StateKey> {}

pub trait StorageView {
    fn view_id(&self) -> StateViewId;

    fn get_storage_usage(&self) -> anyhow::Result<StateStorageUsage>;
}

impl<T: TRawStateView> StorageView for T {
    fn view_id(&self) -> StateViewId {
        self.id()
    }

    fn get_storage_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.get_usage()
    }
}

pub trait StateView:
    TResourceView<Key = StateKey> + TModuleView<Key = StateKey> + StorageView
{
}

impl<T: TResourceView<Key = StateKey> + TModuleView<Key = StateKey> + StorageView> StateView for T {}

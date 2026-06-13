module aptos_trading::orders_store {
    use aptos_trading::native_store_capability::NativeStoreCapability;
    use aptos_trading::single_order_types::SingleOrder;
    use aptos_trading::bulk_order_types::BulkOrder;
    use aptos_trading::order_book_types::OrderId;


    native fun set_single_order(capability: &NativeStoreCapability, market: address, order: SingleOrder);
    native fun delete_single_order(capability: &NativeStoreCapability, market: address, order_id: OrderId);

    native fun set_bulk_order(capability: &NativeStoreCapability, market: address, order: BulkOrder);
    native fun delete_bulk_order(capability: &NativeStoreCapability, market: address, order_owner: address);
}

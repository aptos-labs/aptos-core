// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::StorageAdapter,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
    system_module_names::{MULTISIG_ACCOUNT_MODULE, VALIDATE_MULTISIG_TRANSACTION},
    transaction_metadata::TransactionMetadata,
};
use aptos_framework::RuntimeModuleMetadataV1;
use aptos_gas::{
    AbstractValueSizeGasParameters, AptosGasParameters, FromOnChainGasSchedule, Gas,
    NativeGasParameters, StorageGasParameters,
};
use aptos_logger::{enabled, prelude::*, Level};
use aptos_state_view::StateView;
use aptos_types::{
    account_config::{TransactionValidation, APTOS_TRANSACTION_VALIDATION, CORE_CODE_ADDRESS},
    chain_id::ChainId,
    on_chain_config::{
        ApprovedExecutionHashes, ConfigurationResource, FeatureFlag, Features, GasSchedule,
        GasScheduleV2, OnChainConfig, StorageGasSchedule, TimedFeatures, Version,
    },
    transaction::{AbortInfo, Multisig},
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use fail::fail_point;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{
    language_storage::ModuleId,
    move_resource::MoveStructType,
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::logging::expect_no_verification_errors;
use move_vm_types::gas::UnmeteredGasMeter;
use std::sync::Arc;

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::entry_point_trait::PreBuiltPackages;
use aptos_framework::{
    natives::code::PackageMetadata, KnownAttribute, APTOS_METADATA_KEY, APTOS_METADATA_KEY_V1,
};
use aptos_sdk::{
    bcs,
    move_types::{identifier::Identifier, language_storage::ModuleId},
    transaction_builder::aptos_stdlib,
    types::{
        account_address::AccountAddress,
        transaction::{Script, TransactionPayload},
    },
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledScript, FunctionHandleIndex, IdentifierIndex, SignatureToken},
    CompiledModule,
};
use rand::{rngs::StdRng, Rng};

// Information used to track a publisher and what allows to identify and
// version the package published.
#[derive(Clone, Debug)]
struct PublisherInfo {
    publisher: AccountAddress,
    #[allow(dead_code)]
    suffix: u64,
    fn_count: usize,
}

// Given a Package, track all publishers.
#[derive(Clone, Debug)]
struct PackageTracker {
    publishers: Vec<PublisherInfo>,
    suffix: u64,
    package: Package,
}

impl PackageTracker {
    fn find_info(&self, publisher: &AccountAddress) -> Option<usize> {
        self.publishers
            .iter()
            .position(|info| &info.publisher == publisher)
    }
}

// Holds all the packages known and return a proper Package to be used.
#[derive(Clone, Debug)]
pub struct PackageHandler {
    packages: Vec<PackageTracker>,
    is_simple: bool,
}

impl PackageHandler {
    pub fn new(pre_built: &'static dyn PreBuiltPackages, name: &str) -> Self {
        let packages = vec![PackageTracker {
            publishers: vec![],
            suffix: 0,
            package: Package::by_name(pre_built, name),
        }];
        PackageHandler {
            packages,
            is_simple: name == "simple",
        }
    }

    // Return a `Package` to be published. Packages are tracked by publisher so if
    // the same `LocalAccount` is used, the package will be an upgrade of the existing one
    // otherwise a "new" package will be generated (new suffix)
    pub fn pick_package(&mut self, rng: &mut StdRng, publisher_address: AccountAddress) -> Package {
        let idx = rng.gen_range(0usize, self.packages.len());
        let tracker = self
            .packages
            .get_mut(idx)
            .expect("PackageTracker must exisit");
        let (idx, version) = match tracker.find_info(&publisher_address) {
            Some(idx) => (idx, true),
            None => {
                let fn_count = rng.gen_range(0usize, 30usize);
                tracker.publishers.push(PublisherInfo {
                    publisher: publisher_address,
                    suffix: tracker.suffix,
                    fn_count,
                });
                tracker.suffix += 1;
                (tracker.publishers.len() - 1, false)
            },
        };
        let mut package = tracker.package.update(
            tracker.publishers[idx].publisher,
            0,
            // TODO cleanup.
            // unnecessary to have indices for module published under different accout,
            // they can all be named the same
        );
        if self.is_simple {
            if version {
                update_simple_move_version(package.get_mut_module("simple"));
            }
            scramble_simple_move(
                package.get_mut_module("simple"),
                tracker.publishers[idx].fn_count,
                rng,
            );
        }
        package
    }
}

// Enum to define all packages known to the publisher code.
#[derive(Clone, Debug)]
pub enum Package {
    Simple(
        Vec<(String, CompiledModule)>,
        PackageMetadata,
        Option<CompiledScript>,
    ),
}

impl Package {
    pub fn by_name(pre_built: &'static dyn PreBuiltPackages, name: &str) -> Self {
        let (modules, metadata) = Self::load_package(
            pre_built.package_metadata(name),
            pre_built.package_modules(name),
        );
        let script = pre_built
            .package_script(name)
            .map(|code| CompiledScript::deserialize(code).expect("Script must deserialize"));
        Self::Simple(modules, metadata, script)
    }

    pub fn script(&self, publisher: AccountAddress) -> TransactionPayload {
        match self {
            Self::Simple(_, _, script_opt) => {
                let mut script = script_opt
                    .clone()
                    .expect("Script not defined for wanted package");
                assert_ne!(publisher, AccountAddress::MAX_ADDRESS);

                // Make sure dependencies link to published modules. Compiler V2 adds 0xf..ff so we need to
                // skip it.
                assert_eq!(script.address_identifiers.len(), 2);
                for address in &mut script.address_identifiers {
                    if address != &AccountAddress::MAX_ADDRESS {
                        *address = publisher;
                    }
                }

                let mut code = vec![];
                script.serialize(&mut code).expect("Script must serialize");
                TransactionPayload::Script(Script::new(code, vec![], vec![]))
            },
        }
    }

    fn load_package(
        package_bytes: &[u8],
        modules_bytes: &[Vec<u8>],
    ) -> (Vec<(String, CompiledModule)>, PackageMetadata) {
        let metadata = bcs::from_bytes::<PackageMetadata>(package_bytes)
            .expect("PackageMetadata for GenericModule must deserialize");
        let mut modules = Vec::new();
        for module_content in modules_bytes {
            let module =
                CompiledModule::deserialize(module_content).expect("Simple.move must deserialize");
            modules.push((module.self_id().name().to_string(), module));
        }
        (modules, metadata)
    }

    // Given an "original" package, updates all modules with the given publisher.
    pub fn update(&self, publisher: AccountAddress, suffix: u64) -> Self {
        match self {
            Self::Simple(modules, metadata, script) => {
                let (new_modules, metadata) = update(modules, metadata, publisher, suffix);
                Self::Simple(new_modules, metadata, script.clone())
            },
        }
    }

    pub fn get_publish_args(&self) -> (Vec<u8>, Vec<Vec<u8>>) {
        match self {
            Self::Simple(modules, metadata, _) => {
                let metadata_serialized =
                    bcs::to_bytes(metadata).expect("PackageMetadata must serialize");
                let mut code: Vec<Vec<u8>> = vec![];
                for (_, module) in modules {
                    let mut module_code: Vec<u8> = vec![];
                    module
                        .serialize(&mut module_code)
                        .expect("Module must serialize");
                    code.push(module_code);
                }
                (metadata_serialized, code)
            },
        }
    }

    // Return a transaction payload to publish the current package
    pub fn publish_transaction_payload(&self) -> TransactionPayload {
        let (metadata_serialized, code) = self.get_publish_args();
        aptos_stdlib::code_publish_package_txn(metadata_serialized, code)
    }

    pub fn get_module_id(&self, module_name: &str) -> ModuleId {
        match self {
            Self::Simple(modules, _, _) => {
                for (name, module) in modules {
                    if name == module_name {
                        return module.self_id();
                    }
                }
                panic!("Module for {} not found", module_name);
            },
        }
    }

    pub fn get_mut_module(&mut self, module_name: &str) -> &mut CompiledModule {
        match self {
            Self::Simple(modules, _, _) => {
                for (name, module) in modules {
                    if name == module_name {
                        return module;
                    }
                }
                panic!("Module for {} not found", module_name);
            },
        }
    }
}

fn update(
    modules: &[(String, CompiledModule)],
    metadata: &PackageMetadata,
    publisher: AccountAddress,
    suffix: u64,
) -> (Vec<(String, CompiledModule)>, PackageMetadata) {
    let mut new_modules = Vec::new();
    for (original_name, module) in modules {
        let mut new_module = module.clone();
        let module_handle = new_module
            .module_handles
            .get(module.self_handle_idx().0 as usize)
            .expect("ModuleId for self must exists");
        let original_address_idx = module_handle.address.0;
        let original_address = new_module.address_identifiers[original_address_idx as usize];
        let _ = std::mem::replace(
            &mut new_module.address_identifiers[original_address_idx as usize],
            publisher,
        );

        for constant in new_module.constant_pool.iter_mut() {
            if constant.type_ == SignatureToken::Address
                && original_address == AccountAddress::from_bytes(constant.data.clone()).unwrap()
            {
                constant.data.swap_with_slice(&mut publisher.to_vec());
            }
        }

        if suffix > 0 {
            for module_handle in &new_module.module_handles {
                if module_handle.address.0 == original_address_idx {
                    let mut new_name =
                        new_module.identifiers[module_handle.name.0 as usize].to_string();
                    new_name.push('_');
                    new_name.push_str(suffix.to_string().as_str());
                    let _ = std::mem::replace(
                        &mut new_module.identifiers[module_handle.name.0 as usize],
                        Identifier::new(new_name).expect("Identifier must be legal"),
                    );
                }
            }
        }
        if let Some(mut metadata) = aptos_framework::get_metadata_from_compiled_module(&new_module)
        {
            metadata
                .struct_attributes
                .iter_mut()
                .for_each(|(_, attrs)| {
                    attrs.iter_mut().for_each(|attr| {
                        if let Some(member) = attr.get_resource_group_member() {
                            if member.module_id().address() == &original_address {
                                let new_full_name = format!(
                                    "{}::{}::{}",
                                    publisher.to_standard_string(),
                                    member.module,
                                    member.name
                                );
                                let _ = std::mem::replace(
                                    attr,
                                    KnownAttribute::resource_group_member(new_full_name),
                                );
                            }
                        }
                    });
                });
            let mut count = 0;
            new_module.metadata.iter_mut().for_each(|metadata_holder| {
                if metadata_holder.key == APTOS_METADATA_KEY_V1
                    || metadata_holder.key == APTOS_METADATA_KEY
                {
                    metadata_holder.value =
                        bcs::to_bytes(&metadata).expect("Metadata must serialize");
                    count += 1;
                }
            });
            assert!(count == 1, "{:?}", new_module.metadata);
        }

        new_modules.push((original_name.clone(), new_module));
    }
    let mut metadata = metadata.clone();
    if suffix > 0 {
        for module in &mut metadata.modules {
            let mut new_name = module.name.clone();
            new_name.push('_');
            new_name.push_str(suffix.to_string().as_str());
            module.name = new_name;
        }
    }
    (new_modules, metadata)
}

//
// Functions to load and update the original package
//

fn update_simple_move_version(module: &mut CompiledModule) {
    // change `const COUNTER_STEP` in Simple.move
    // That is the only u64 in the constant pool
    for constant in &mut module.constant_pool {
        if constant.type_ == SignatureToken::U64 {
            let mut v: u64 = bcs::from_bytes(&constant.data).expect("U64 must deserialize");
            v += 1;
            constant.data = bcs::to_bytes(&v).expect("U64 must serialize");
            break;
        }
    }
}

fn scramble_simple_move(module: &mut CompiledModule, fn_count: usize, rng: &mut StdRng) {
    // change `const RANDOM` in Simple.move
    // That is the only vector<u64> in the constant pool
    let const_len = rng.gen_range(0usize, 5000usize);
    let mut v = Vec::<u64>::with_capacity(const_len);
    for i in 0..const_len {
        v.push(i as u64);
    }
    // module.constant_pool
    for constant in &mut module.constant_pool {
        if constant.type_ == SignatureToken::Vector(Box::new(SignatureToken::U64)) {
            constant.data = bcs::to_bytes(&v).expect("U64 vector must serialize");
            break;
        }
    }

    // find the copy_pasta* function in Simple.move
    let mut def = None;
    let mut handle = None;
    let mut func_name = String::new();
    for func_def in &module.function_defs {
        let func_handle = &module.function_handles[func_def.function.0 as usize];
        let name = module.identifiers[func_handle.name.0 as usize].as_str();
        if name.starts_with("copy_pasta") {
            def = Some(func_def.clone());
            handle = Some(func_handle.clone());
            func_name = String::from(name);
            break;
        }
    }
    if let Some(fd) = def {
        for suffix in 0..fn_count {
            let mut func_handle = handle.clone().expect("Handle must be defined");
            let mut func_def = fd.clone();
            let mut name = func_name.clone();
            name.push_str(suffix.to_string().as_str());
            module
                .identifiers
                .push(Identifier::new(name.as_str()).expect("Identifier name must be valid"));
            func_handle.name = IdentifierIndex((module.identifiers.len() - 1) as u16);
            module.function_handles.push(func_handle);
            func_def.function = FunctionHandleIndex((module.function_handles.len() - 1) as u16);
            module.function_defs.push(func_def);
        }
    }
}

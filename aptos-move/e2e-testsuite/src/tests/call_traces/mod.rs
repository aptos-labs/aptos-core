use aptos_types::transaction::{Module, SignedTransaction};
use language_e2e_tests::account::AccountData;
use language_e2e_tests::compile::compile_script;
use move_deps::move_binary_format::CompiledModule;
use move_deps::move_bytecode_verifier::verify_module;
use move_deps::move_ir_compiler::Compiler;

mod test;

fn add_set_message_txn(sender: &AccountData, seq_num: u64) -> (CompiledModule, SignedTransaction) {
    let module_code = format!(
        "
        module 0x{}.M {{
            import 0x1.signer;

            struct Number has key {{ number: u64 }}

            public set_number(account: &signer, number: u64) {{
            label b0:
                move_to<Number>(move(account), Number {{ number: move(number) }});
                return;
            }}
        }}
        ",
        sender.address(),
    );

    let framework_modules = cached_packages::head_release_bundle().compiled_modules();
    let compiler = Compiler {
        deps: framework_modules.iter().collect(),
    };
    let module = compiler
        .into_compiled_module(module_code.as_str())
        .expect("Module compilation failed");
    let mut module_blob = vec![];
    module
        .serialize(&mut module_blob)
        .expect("Module must serialize");
    verify_module(&module).expect("Module must verify");
    (
        module,
        sender
            .account()
            .transaction()
            .module(Module::new(module_blob))
            .sequence_number(seq_num)
            .sign(),
    )
}

fn call_set_message_txn(
    sender: &AccountData,
    seq_num: u64,
    extra_deps: Vec<CompiledModule>,
    the_number: u64,
) -> SignedTransaction {
    let program = format!(
        "
            import 0x{}.M;

            main(account: signer) {{
            label b0:
                M.set_number(&account, {});
                return;
            }}
        ",
        sender.address(),
        the_number,
    );

    let module = compile_script(&program, extra_deps);
    sender
        .account()
        .transaction()
        .script(module)
        .sequence_number(seq_num)
        .sign()
}

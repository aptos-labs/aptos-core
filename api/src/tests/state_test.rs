// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{current_function_name, tests::new_test_context};

#[tokio::test]
async fn test_get_account_resource() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .get(&get_account_resource("0xA550C18", "0x1::GUID::Generator"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_resource_by_invalid_address() {
    let mut context = new_test_context(current_function_name!());
    let invalid_addresses = vec!["1", "0xzz", "01"];
    for invalid_address in &invalid_addresses {
        let resp = context
            .expect_status_code(400)
            .get(&get_account_resource(
                invalid_address,
                "0x1::GUID::Generator",
            ))
            .await;
        context.check_golden_output(resp);
    }
}

#[tokio::test]
async fn test_get_account_resource_by_invalid_struct_tag() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&get_account_resource("0xA550C18", "0x1::GUID_Generator"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_resource_address_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_resource("0xA550C19", "0x1::GUID::Generator"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_resource_struct_tag_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_resource("0xA550C19", "0x1::GUID::GeneratorX"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_module() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get(&get_account_module("0x1", "GUID")).await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_module_by_invalid_address() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&get_account_module("1", "GUID"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_account_module_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_module("0x1", "NoNoNo"))
        .await;
    context.check_golden_output(resp);
}

fn get_account_resource(address: &str, struct_tag: &str) -> String {
    format!("/accounts/{}/resource/{}", address, struct_tag)
}

fn get_account_module(address: &str, name: &str) -> String {
    format!("/accounts/{}/module/{}", address, name)
}

#![cfg(test)]
extern crate std;
use std::println;

use crate::{contract::Token, storage_types::ProjectInfo, TokenClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger, LedgerInfo},
    vec, Address, Env, IntoVal, Symbol,
};

mod token_contract {
    soroban_sdk::contractimport!(file = "./token/soroban_token_contract.optimized.wasm");
}

mod nft_contract {
    soroban_sdk::contractimport!(file = "./token/non_fungible_token.optimized.wasm");
}

fn create_custom_token<'a>(e: &Env, admin: &Address, decimals: &u32) -> token_contract::Client<'a> {
    let token_id = &e.register_contract_wasm(None, token_contract::WASM);
    let token = token_contract::Client::new(e, &token_id);
    token.initialize(admin, decimals, &"name".into_val(e), &"symbol".into_val(e));
    token
}

fn create_nft<'a>(e: &Env, admin: &Address) -> nft_contract::Client<'a> {
    let nft_id = &e.register_contract_wasm(None, nft_contract::WASM);
    let nft = nft_contract::Client::new(e, &nft_id);
    nft.initialize(admin, &"EnerDAO NFT".into_val(e), &"EnerDAO".into_val(e));
    nft
}

fn create_token<'a>(e: &Env, admin: &Address) -> TokenClient<'a> {
    let token = TokenClient::new(e, &e.register_contract(None, Token {}));
    let random_address: Address = Address::generate(&e);
    let projectInfo = ProjectInfo {
        borrower: random_address.clone(),
        deposit_token_address: random_address.clone(),
        collateral_nft_address: random_address.clone(),
        collateral_id: 0,
        target_amount: 0,
        start_timestamp: 0,
        final_timestamp: 0,
        reward_rate: 0,
    };
    token.initialize(
        admin,
        &7,
        &"name".into_val(e),
        &"symbol".into_val(e),
        &projectInfo
    );
    token
}

#[test]
fn test_deposit() {
    // Here we test usdt token deposit to the contract
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let depositor_2 = Address::generate(&e);
    let borrower = Address::generate(&e);
    let usdt_token = create_token(&e, &admin);

    usdt_token.mint(&depositor, &3000_0000000i128);
    usdt_token.mint(&depositor_2, &1000_0000000i128);

    let current_info: LedgerInfo = e.ledger().get();
    let current_timestamp: u64 = current_info.timestamp;

    let project_info = ProjectInfo {
        borrower: borrower.clone(),
        deposit_token_address: usdt_token.address.clone(),
        collateral_nft_address: usdt_token.address.clone(),
        collateral_id: 0,
        target_amount: 3000_0000000i128,
        start_timestamp: current_timestamp,
        final_timestamp: current_timestamp + 1000_u64,
        reward_rate: 0,
    };
    
    let contract = TokenClient::new(&e, &e.register_contract(None, Token {}));
    contract.initialize(
        &admin,
        &7,
        &"LP EnerDAO".into_val(&e),
        &"LPE".into_val(&e),
        &project_info,
    );

    assert_eq!(contract.total_supply(), 0);
    assert_eq!(contract.balance(&depositor), 0);
    assert_eq!(usdt_token.balance(&depositor), 3000_0000000i128);
    assert_eq!(contract.number_of_depositors(), 0);

    contract.deposit(&depositor, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 1000_0000000i128);
    assert_eq!(contract.balance(&depositor), 1000_0000000i128);
    assert_eq!(usdt_token.balance(&depositor), 2000_0000000i128);
    assert_eq!(contract.number_of_depositors(), 1);

    contract.deposit(&depositor, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 2000_0000000i128);
    assert_eq!(contract.balance(&depositor), 2000_0000000i128);
    assert_eq!(usdt_token.balance(&depositor), 1000_0000000i128);
    assert_eq!(contract.number_of_depositors(), 1);

    contract.deposit(&depositor_2, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 3000_0000000i128);
    assert_eq!(contract.balance(&depositor), 2000_0000000i128);
    assert_eq!(usdt_token.balance(&depositor), 1000_0000000i128);
    assert_eq!(contract.balance(&depositor_2), 1000_0000000i128);
    assert_eq!(usdt_token.balance(&depositor_2), 0);
    assert_eq!(contract.number_of_depositors(), 2);

    assert_eq!(contract.get_depositors(), vec![&e, depositor, depositor_2]);
}

// #[test]
// fn test_borrower_return() {
//     // Here we test usdt borrower return to the contract
//     let e = Env::default();
//     e.mock_all_auths();

//     let admin = Address::generate(&e);
//     let depositor = Address::generate(&e);
//     let depositor_2 = Address::generate(&e);
//     let borrower = Address::generate(&e);
//     let usdt_token = create_custom_token(&e, &admin, &7);

//     usdt_token.mint(&depositor, &2000_0000000i128);
//     usdt_token.mint(&depositor_2, &1000_0000000i128);

//     let current_timestamp: u64 = std::time::SystemTime::now()
//         .duration_since(std::time::SystemTime::UNIX_EPOCH)
//         .unwrap()
//         .as_secs();

//     let contract = TokenClient::new(&e, &e.register_contract(None, Token {}));
//     contract.initialize(
//         &admin,
//         &7,
//         &"LP EnerDAO".into_val(&e),
//         &"LPE".into_val(&e),
//         &borrower,
//         &usdt_token.address,
//         &3000_0000000i128,
//         &(current_timestamp + 1000_u64),
//     );

//     assert_eq!(contract.total_supply(), 0);
//     assert_eq!(contract.balance(&depositor), 0);
//     assert_eq!(usdt_token.balance(&depositor), 2000_0000000i128);
//     assert_eq!(contract.number_of_depositors(), 0);

//     contract.deposit(&depositor, &2000_0000000i128);

//     assert_eq!(contract.total_supply(), 2000_0000000i128);
//     assert_eq!(contract.balance(&depositor), 2000_0000000i128);
//     assert_eq!(usdt_token.balance(&depositor), 0);
//     assert_eq!(contract.number_of_depositors(), 1);

//     contract.deposit(&depositor_2, &1000_0000000i128);

//     assert_eq!(usdt_token.balance(&contract.address), 3000_0000000i128);
//     assert_eq!(contract.total_supply(), 3000_0000000i128);
//     assert_eq!(contract.balance(&depositor), 2000_0000000i128);
//     assert_eq!(usdt_token.balance(&depositor), 0);
//     assert_eq!(contract.balance(&depositor_2), 1000_0000000i128);
//     assert_eq!(usdt_token.balance(&depositor_2), 0);
//     assert_eq!(contract.number_of_depositors(), 2);

//     let mut current_info: LedgerInfo = e.ledger().get();
//     current_info.timestamp = current_timestamp + 1001_u64;
//     e.ledger().set(current_info);

//     assert_eq!(usdt_token.balance(&borrower), 0);
//     contract.borrower_claim();
//     assert_eq!(usdt_token.balance(&borrower), 3000_0000000i128);
//     assert_eq!(usdt_token.balance(&contract.address), 0);

//     contract.borrower_return(&borrower, &1000_0000000i128);
//     contract.borrower_return(&borrower, &500_0000000i128);
//     contract.borrower_return(&borrower, &1500_0000000i128);

//     assert_eq!(usdt_token.balance(&borrower), 0);
//     assert_eq!(contract.balance(&depositor), 2000_0000000i128);
//     assert_eq!(contract.balance(&depositor_2), 1000_0000000i128);
// }

// #[test]
// fn test_borrower_return_budget() {
//     // Here we test usdt borrower return to the contract
//     let e = Env::default();
//     e.mock_all_auths();

//     let admin = Address::generate(&e);

//     let borrower = Address::generate(&e);
//     let usdt_token = create_custom_token(&e, &admin, &7);

//     let current_timestamp: u64 = std::time::SystemTime::now()
//         .duration_since(std::time::SystemTime::UNIX_EPOCH)
//         .unwrap()
//         .as_secs();

//     // let contract = TokenClient::new(&e, &e.register_contract(None, Token {}));

//     mod wasm_contract {
//         soroban_sdk::contractimport!(
//             file = "./target/wasm32-unknown-unknown/release/enerdao_token_contract.optimized.wasm"
//         );
//     }
//     let contract_id = &e.register_contract_wasm(None, wasm_contract::WASM);
//     let contract = TokenClient::new(&e, &contract_id);

//     contract.initialize(
//         &admin,
//         &7,
//         &"LP EnerDAO".into_val(&e),
//         &"LPE".into_val(&e),
//         &borrower,
//         &usdt_token.address,
//         &3000_0000000i128,
//         &(current_timestamp + 1000_u64),
//     );

//     e.budget().reset_unlimited();
//     for _ in 0..10 {
//         let depositor = Address::generate(&e);
//         usdt_token.mint(&depositor, &1000_0000000i128);
//         contract.deposit(&depositor, &1000_0000000i128);
//     }

//     assert_eq!(usdt_token.balance(&contract.address), 10_000_0000000i128);

//     let mut current_info: LedgerInfo = e.ledger().get();
//     current_info.timestamp = current_timestamp + 1001_u64;
//     e.ledger().set(current_info);

//     contract.borrower_claim();

//     e.budget().reset_unlimited();
//     contract.borrower_return(&borrower, &1000_0000000i128);
//     println!(
//         "      return to 10 depositors: {:?}",
//         e.budget().cpu_instruction_cost()
//     );
// }

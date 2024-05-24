#![cfg(test)]
extern crate std;
use std::println;

use crate::{contract::EnerDAOToken, contract::EnerDAOTokenClient, storage_types::ProjectInfo};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger, LedgerInfo},
    token, vec, Address, Env, IntoVal, String,
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

use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

#[test]
fn test_lend() {
    // Here we test eurc token lend to the contract
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let lender = Address::generate(&e);
    let lender_2 = Address::generate(&e);
    let borrower = Address::generate(&e);
    let (eurc_token, eurc_admin) = create_token(&e, &admin);

    eurc_admin.mint(&lender, &3000_0000000i128);
    eurc_admin.mint(&lender_2, &1000_0000000i128);

    let current_info: LedgerInfo = e.ledger().get();
    let current_timestamp: u64 = current_info.timestamp;

    let project_info = ProjectInfo {
        borrower: borrower.clone(),
        lend_token_address: eurc_token.address.clone(),
        collateral_nft_address: eurc_token.address.clone(),
        collateral_id: 0,
        target_amount: 3000_0000000i128,
        start_timestamp: current_timestamp,
        final_timestamp: current_timestamp + 1000_u64,
        reward_rate: 0,
        treasury_address: admin.clone(),
    };

    let contract = EnerDAOTokenClient::new(&e, &e.register_contract(None, EnerDAOToken {}));
    contract.initialize(&admin, &7, &"LP EnerDAO".into_val(&e), &"LPE".into_val(&e));

    contract.init_project(
        &project_info.borrower,
        &project_info.lend_token_address,
        &project_info.collateral_nft_address,
        &project_info.collateral_id,
        &project_info.target_amount,
        &project_info.start_timestamp,
        &project_info.final_timestamp,
        &project_info.reward_rate,
        &project_info.treasury_address,
    );

    assert_eq!(contract.total_supply(), 0);
    assert_eq!(contract.balance(&lender), 0);
    assert_eq!(eurc_token.balance(&lender), 3000_0000000i128);
    assert_eq!(contract.number_of_lenders(), 0);

    contract.lend(&lender, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 1000_0000000i128);
    assert_eq!(contract.balance(&lender), 1000_0000000i128);
    assert_eq!(eurc_token.balance(&lender), 2000_0000000i128);
    assert_eq!(contract.number_of_lenders(), 1);

    contract.lend(&lender, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 2000_0000000i128);
    assert_eq!(contract.balance(&lender), 2000_0000000i128);
    assert_eq!(eurc_token.balance(&lender), 1000_0000000i128);
    assert_eq!(contract.number_of_lenders(), 1);

    contract.lend(&lender_2, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 3000_0000000i128);
    assert_eq!(contract.balance(&lender), 2000_0000000i128);
    assert_eq!(eurc_token.balance(&lender), 1000_0000000i128);
    assert_eq!(contract.balance(&lender_2), 1000_0000000i128);
    assert_eq!(eurc_token.balance(&lender_2), 0);
    assert_eq!(contract.number_of_lenders(), 2);

    assert_eq!(contract.get_lenders(), vec![&e, lender, lender_2]);
}

#[test]
fn test_borrower_return() {
    // Here we test eurc borrower return to the contract
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let lender = Address::generate(&e);
    let lender_2 = Address::generate(&e);
    let borrower = Address::generate(&e);
    let eurc_token = create_custom_token(&e, &admin, &7);

    eurc_token.mint(&lender, &1000_0000000i128);
    eurc_token.mint(&lender_2, &1000_0000000i128);

    let nft = create_nft(&e, &admin);

    let current_info: LedgerInfo = e.ledger().get();
    let current_timestamp: u64 = current_info.timestamp;

    let project_info = ProjectInfo {
        borrower: borrower.clone(),
        lend_token_address: eurc_token.address.clone(),
        collateral_nft_address: nft.address.clone(),
        collateral_id: 0,
        target_amount: 2000_0000000i128,
        start_timestamp: current_timestamp,
        final_timestamp: current_timestamp + 1000_u64,
        reward_rate: 1000,
        treasury_address: admin.clone(),
    };

    let contract = EnerDAOTokenClient::new(&e, &e.register_contract(None, EnerDAOToken {}));
    contract.initialize(&admin, &7, &"LP EnerDAO".into_val(&e), &"LPE".into_val(&e));

    contract.init_project(
        &project_info.borrower,
        &project_info.lend_token_address,
        &project_info.collateral_nft_address,
        &project_info.collateral_id,
        &project_info.target_amount,
        &project_info.start_timestamp,
        &project_info.final_timestamp,
        &project_info.reward_rate,
        &project_info.treasury_address,
    );

    assert_eq!(contract.total_supply(), 0);
    assert_eq!(contract.balance(&lender), 0);
    assert_eq!(eurc_token.balance(&lender), 1000_0000000i128);
    assert_eq!(contract.number_of_lenders(), 0);

    contract.lend(&lender, &1000_0000000i128);

    assert_eq!(contract.total_supply(), 1000_0000000i128);
    assert_eq!(contract.balance(&lender), 1000_0000000i128);
    assert_eq!(eurc_token.balance(&lender), 0);
    assert_eq!(contract.number_of_lenders(), 1);

    contract.lend(&lender_2, &1000_0000000i128);

    assert_eq!(eurc_token.balance(&contract.address), 2000_0000000i128);
    assert_eq!(contract.total_supply(), 2000_0000000i128);
    assert_eq!(contract.balance(&lender), 1000_0000000i128);
    assert_eq!(eurc_token.balance(&lender), 0);
    assert_eq!(contract.balance(&lender_2), 1000_0000000i128);
    assert_eq!(eurc_token.balance(&lender_2), 0);
    assert_eq!(contract.number_of_lenders(), 2);

    let mut current_info: LedgerInfo = e.ledger().get();
    current_info.timestamp = current_timestamp + 1001_u64;
    e.ledger().set(current_info);

    assert_eq!(contract.borrower_claim_status(), String::from_str(&e, "NoCollateral"));

    nft.mint(
        &contract.address,
        &0,
        &String::from_str(&e, "https://uri.com"),
    );

    assert_eq!(eurc_token.balance(&borrower), 0);
    assert_eq!(contract.borrower_claim_status(), String::from_str(&e, "Available"));
    contract.borrower_claim();
    assert_eq!(contract.borrower_claim_status(), String::from_str(&e, "AlreadyClaimed"));
    // assert_eq!(eurc_token.balance(&borrower), 2000_0000000i128);
    // assert_eq!(eurc_token.balance(&contract.address), 0);

    contract.borrower_return(&borrower, &1110_0000000i128);
    assert_eq!(contract.lender_available_to_claim(&lender), 550_0000000i128);
    assert_eq!(
        contract.lender_available_to_claim(&lender_2),
        550_0000000i128
    );


    eurc_token.mint(&borrower, &220_0000000i128);
    contract.borrower_return(&borrower, &1110_0000000i128);
    assert_eq!(
        contract.lender_available_to_claim(&lender),
        1100_0000000i128
    );
    assert_eq!(
        contract.lender_available_to_claim(&lender_2),
        1100_0000000i128
    );
    
    assert_eq!(eurc_token.balance(&contract.address), 2200_0000000i128); // return - protocol fee
    assert_eq!(eurc_token.balance(&admin), 20_0000000i128);              // protocol fee

}

#[test]
fn test_borrower_return_rounding() {
    // Here we test eurc borrower return to the contract
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let lender = Address::generate(&e);
    let lender_2 = Address::generate(&e);
    let borrower = Address::generate(&e);
    let eurc_token = create_custom_token(&e, &admin, &7);

    eurc_token.mint(&lender, &2000_0000000i128);
    eurc_token.mint(&lender_2, &1000_0000000i128);

    let nft = create_nft(&e, &admin);

    let current_info: LedgerInfo = e.ledger().get();
    let current_timestamp: u64 = current_info.timestamp;

    let project_info = ProjectInfo {
        borrower: borrower.clone(),
        lend_token_address: eurc_token.address.clone(),
        collateral_nft_address: nft.address.clone(),
        collateral_id: 777,
        target_amount: 3000_0000000i128,
        start_timestamp: current_timestamp,
        final_timestamp: current_timestamp + 1000_u64,
        reward_rate: 1000,
        treasury_address: admin.clone(),
    };

    let contract = EnerDAOTokenClient::new(&e, &e.register_contract(None, EnerDAOToken {}));
    contract.initialize(&admin, &7, &"LP EnerDAO".into_val(&e), &"LPE".into_val(&e));

    contract.init_project(
        &project_info.borrower,
        &project_info.lend_token_address,
        &project_info.collateral_nft_address,
        &project_info.collateral_id,
        &project_info.target_amount,
        &project_info.start_timestamp,
        &project_info.final_timestamp,
        &project_info.reward_rate,
        &project_info.treasury_address,
    );

    contract.lend(&lender, &1000_0000000i128);
    contract.lend(&lender_2, &1000_0000000i128);
    contract.lend(&lender, &1000_0000000i128);

    let mut current_info: LedgerInfo = e.ledger().get();
    current_info.timestamp = current_timestamp + 1001_u64;
    e.ledger().set(current_info);

    nft.mint(
        &contract.address,
        &777,
        &String::from_str(&e, "https://uri.com"),
    );

    contract.borrower_claim();
    assert_eq!(eurc_token.balance(&borrower), 3000_0000000i128);
    assert_eq!(eurc_token.balance(&contract.address), 0);

    contract.borrower_return(&borrower, &1000_0000000i128);
    assert_eq!(contract.lender_available_to_claim(&lender), 660_6606606);
    assert_eq!(contract.lender_available_to_claim(&lender_2), 330_3303303);
    assert_eq!(eurc_token.balance(&contract.address), 990_9909909); // return - protocol fee
    assert_eq!(eurc_token.balance(&admin), 9_0090091);              // protocol fee

    contract.lender_claim(&lender_2);
    assert_eq!(contract.lender_available_to_claim(&lender), 660_6606606);
    assert_eq!(contract.lender_available_to_claim(&lender_2), 0);
}

#[test]
fn test_failed_target_amount() {
    // Here we test eurc borrower return to the contract
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let lender = Address::generate(&e);
    let lender_2 = Address::generate(&e);
    let borrower = Address::generate(&e);
    let eurc_token = create_custom_token(&e, &admin, &7);

    eurc_token.mint(&lender, &2000_0000000i128);
    eurc_token.mint(&lender_2, &1000_0000000i128);

    let nft = create_nft(&e, &admin);

    let current_info: LedgerInfo = e.ledger().get();
    let current_timestamp: u64 = current_info.timestamp;

    let mut project_info = ProjectInfo {
        borrower: borrower.clone(),
        lend_token_address: eurc_token.address.clone(),
        collateral_nft_address: nft.address.clone(),
        collateral_id: 666,
        target_amount: 5000_0000000i128,
        start_timestamp: current_timestamp,
        final_timestamp: current_timestamp + 1000_u64,
        reward_rate: 1000,
        treasury_address: admin.clone(),
    };

    let contract = EnerDAOTokenClient::new(&e, &e.register_contract(None, EnerDAOToken {}));
    contract.initialize(&admin, &7, &"LP EnerDAO".into_val(&e), &"LPE".into_val(&e));

    contract.init_project(
        &project_info.borrower,
        &project_info.lend_token_address,
        &project_info.collateral_nft_address,
        &project_info.collateral_id,
        &project_info.target_amount,
        &project_info.start_timestamp,
        &project_info.final_timestamp,
        &project_info.reward_rate,
        &project_info.treasury_address,
    );

    contract.lend(&lender, &1000_0000000i128);
    contract.lend(&lender_2, &1000_0000000i128);
    contract.lend(&lender, &1000_0000000i128);

    assert_eq!(contract.borrower_claim_status(), String::from_str(&e, "TargetNotReached"));

    contract.set_lender_claim_available(&true, &true);

    assert_eq!(
        contract.lender_available_to_claim(&lender),
        2000_0000000i128
    );
    assert_eq!(
        contract.lender_available_to_claim(&lender_2),
        1000_0000000i128
    );
    assert_eq!(contract.balance(&lender), 2000_0000000i128);
    assert_eq!(contract.balance(&lender_2), 1000_0000000i128);

    contract.lender_claim(&lender_2);
    assert_eq!(
        contract.lender_available_to_claim(&lender),
        2000_0000000i128
    );
    assert_eq!(contract.lender_available_to_claim(&lender_2), 0);

    contract.lender_claim(&lender);
    assert_eq!(contract.lender_available_to_claim(&lender), 0);
    assert_eq!(contract.lender_available_to_claim(&lender_2), 0);
    assert_eq!(eurc_token.balance(&lender), 2000_0000000i128);
    assert_eq!(eurc_token.balance(&lender_2), 1000_0000000i128);
    assert_eq!(contract.balance(&lender), 0);
    assert_eq!(contract.balance(&lender_2), 0);
}

// #[test]
// fn test_borrower_return_budget() {
//     // Here we test eurc borrower return to the contract
//     let e = Env::default();
//     e.mock_all_auths();

//     let admin = Address::generate(&e);

//     let borrower = Address::generate(&e);
//     let eurc_token = create_custom_token(&e, &admin, &7);

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
//         &eurc_token.address,
//         &3000_0000000i128,
//         &(current_timestamp + 1000_u64),
//     );

//     e.budget().reset_unlimited();
//     for _ in 0..10 {
//         let lender = Address::generate(&e);
//         eurc_token.mint(&lender, &1000_0000000i128);
//         contract.lend(&lender, &1000_0000000i128);
//     }

//     assert_eq!(eurc_token.balance(&contract.address), 10_000_0000000i128);

//     let mut current_info: LedgerInfo = e.ledger().get();
//     current_info.timestamp = current_timestamp + 1001_u64;
//     e.ledger().set(current_info);

//     contract.borrower_claim();

//     e.budget().reset_unlimited();
//     contract.borrower_return(&borrower, &1000_0000000i128);
//     println!(
//         "      return to 10 lenders: {:?}",
//         e.budget().cpu_instruction_cost()
//     );
// }

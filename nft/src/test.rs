// 1. ERC721 function currently available for NFT: mint, transfer, owner_of
#![cfg(test)]

extern crate std;

use crate::{contract::NonFungibleToken, NonFungibleTokenClient};
use soroban_sdk::{
    testutils::{Address as _, Logs},
    Address, Env, IntoVal, String
};

fn create_token<'a>(env: &Env, admin: &Address) -> NonFungibleTokenClient<'a> {
    let token = NonFungibleTokenClient::new(env, &env.register_contract(None, NonFungibleToken {}));
    token.initialize(admin, &"EnerDAO NFT".into_val(env), &"EnerDAO".into_val(env));
    token
}

#[test]
fn test_mint() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let nft = create_token(&env, &admin);

    assert_eq!(nft.has_owner(&1), false);
    assert_eq!(nft.has_owner(&2), false);
    assert_eq!(nft.has_owner(&3), false);

    nft.mint(&user1, &1, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));
    nft.mint(&user2, &2, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));
    nft.mint(&user1, &3, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));

    assert_eq!(nft.has_owner(&1), true);
    assert_eq!(nft.has_owner(&2), true);
    assert_eq!(nft.has_owner(&3), true);

    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_owner_of() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let nft = create_token(&env, &admin);

    nft.mint(&user1, &1, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));

    assert_eq!(nft.owner_of(&1), user1);
    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let nft = create_token(&env, &admin);
    nft.mint(&user1, &1, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));
    assert_eq!(nft.owner_of(&1), user1);
    nft.transfer(&user1, &user2, &1);
    assert_eq!(nft.owner_of(&1), user2);
    nft.transfer(&user2, &user1, &1);
    assert_eq!(nft.owner_of(&1), user1);
    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
#[should_panic(expected = "ID already minted")]
fn seat_already_taken() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let nft = create_token(&env, &admin);
    nft.mint(&user1, &1, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));

    nft.mint(&user2, &1, &String::from_str(&env, "https://music.youtube.com/watch?v=yRVotpLaCD4"));
    std::println!("{}", env.logs().all().join("\n"));
}
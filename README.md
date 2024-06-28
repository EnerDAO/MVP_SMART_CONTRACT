# EnerDAOToken MVP Stellar Soroban Smart Contract

## Overview

EnerDAOToken is a Stellar Soroban smart contract implementing a tokenized funding mechanism for EnerDAO. This contract allows users to participate in funding projects, with features for lending, borrowing, and claiming rewards.

## Key Features

- **Tokenized Lending**: Users can lend tokens to projects.
- **Borrower Functionality**: Project owners can claim funds and return borrowed amounts.
- **Reward System**: Implements a reward mechanism for lenders.
- **NFT Collateral**: Utilizes NFTs as collateral for borrowing.
- **Admin Controls**: Includes administrative functions for contract management.

## Main Functions

### For Lenders

- `lend`: Allows users to lend tokens to the project.
- `lender_claim`: Enables lenders to claim their returns and rewards.
- `lender_available_to_claim`: Checks the amount available for a lender to claim.

### For Borrowers

- `borrower_claim`: Allows the borrower to claim the raised funds.
- `borrower_return`: Enables the borrower to return the borrowed amount plus rewards.
- `borrower_to_payback`: Calculates the amount the borrower needs to pay back.

### Administrative Functions

- `initialize`: Initializes the contract with basic token information.
- `init_project`: Sets up project details including target amount, timelines, and reward rates.
- `set_admin`: Changes the contract administrator.
- `set_project_info`: Updates project information.
- `grant_nft`: Transfers the collateral NFT.
- `rescue_tokens`: Allows the admin to rescue tokens sent to the contract by mistake.

### Token Standard Functions

Implements standard token functions like `transfer`, `approve`, `allowance`, etc.

## Key Concepts

- **Protocol Fee**: A fee charged on returns, calculated based on the reward rate.
- **Reward Rate**: Determines the additional return lenders receive.
- **Target Amount**: The funding goal for the project.
- **NFT Collateral**: An NFT used as collateral for the borrowed funds.

## Usage

To interact with this contract, you'll need to use a Stellar Soroban-compatible wallet or SDK. The contract should be deployed on a Stellar network that supports Soroban.

## Security Considerations

- The contract includes checks for various conditions like project timelines and target amounts.
- Admin functions are protected and can only be called by the designated administrator.
- Ensure proper testing and auditing before deploying in a production environment.

## Development

This contract is written in Rust for the Stellar Soroban platform. To develop or modify this contract:

1. Set up a Rust development environment.
2. Install the Soroban CLI and SDK.
3. Use `cargo build` to compile the contract.
4. Deploy the contract using Soroban deployment tools.

## Testing

The contract includes test functions. Run tests using:

```
cargo test
```

## Disclaimer

This smart contract is provided as-is.

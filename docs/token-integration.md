# Token Integration (USDC)

This document describes how the Credence bond contract integrates with Stellar token contracts for USDC-denominated bonds.

## Overview

The bond contract uses Soroban token interfaces for all value movements:

- `initialize(admin, token)` stores the custody token contract address.
- `create_bond` and `top_up` move tokens from identity to contract with `transfer_from`.
- `withdraw` moves tokens from contract to the bonded identity with `transfer`.
- `withdraw_early` moves net proceeds to the bonded identity and the penalty to treasury with `transfer`.

## Contract API

- `initialize(admin, token)`
  - Stores the custody token during contract setup.
- `get_token()`
  - Returns the currently configured token address.

## Security Model

Token handling is centralized in `contracts/credence_bond/src/token_integration.rs` with the following controls:

1. **Admin-gated token configuration**
   - Only stored admin can set token address.
2. **Allowance pre-checks**
   - Before `transfer_from`, contract checks `allowance(owner, contract)`.
   - If allowance is insufficient, call fails with `insufficient token allowance`.
3. **Positive amount validation**
   - `create_bond`, `top_up`, `withdraw`, and `withdraw_early` reject `amount <= 0`.
4. **Checks-effects-interactions**
   - Exit paths persist the reduced bond state before transferring tokens out.
5. **Single integration layer**
   - Prevents duplicated transfer logic and keeps security review surface small.

## Assumptions

- Admin initializes the contract with a valid token contract address.
- Identity accounts grant approvals to the bond contract before `create_bond` and `top_up`.
- Token contract adheres to Soroban token interface semantics.

## Test Coverage (Integration-Specific)

Root custody tests cover:

- Token configuration and retrieval.
- Successful token movement into contract during `create_bond`.
- Failure on missing allowance for `create_bond`.
- Failure when `top_up` exceeds remaining allowance.
- Successful token movement back to identity on `withdraw`.
- Treasury and identity routing during `withdraw_early`.

Run targeted tests:

```bash
cargo test -p credence_bond token_integration_test -- --nocapture
```

Run full package tests:

```bash
cargo test -p credence_bond -- --nocapture
```

## Custody Invariant

For unslashed flows, contract token custody should match the withdrawable bond amount:

```text
token.balance(bond_contract) == bonded_amount - slashed_amount
```

See [bond-token-custody.md](bond-token-custody.md) for the current scope and the remaining slash-path gap.

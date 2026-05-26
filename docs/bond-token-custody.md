# Bond Token Custody

`CredenceBond` escrows the configured token inside the bond contract instead of tracking phantom balances in storage alone.

## Custody Rules

- `initialize(admin, token)` stores the token contract address used for custody.
- `create_bond(identity, amount, ...)` pulls `amount` from `identity` into the bond contract with `transfer_from`.
- `top_up(amount)` pulls additional approved tokens from the bonded identity into the bond contract.
- `withdraw(amount)` transfers `amount` from the bond contract back to the bonded identity after state is reduced.
- `withdraw_early(amount)` transfers `amount - penalty` to the bonded identity and `penalty` to the configured treasury after state is reduced.

## Authorization and Allowance

- `create_bond`, `top_up`, `withdraw`, and `withdraw_early` require `identity.require_auth()`.
- `create_bond` and `top_up` require a token allowance for the bond contract address.
- If allowance is below the requested pull amount, the call reverts with `insufficient token allowance`.

## Current Invariant

For unslashed flows, the custody invariant is:

```text
token.balance(bond_contract) == bonded_amount - slashed_amount
```

This now holds across `create_bond`, `top_up`, `withdraw`, and `withdraw_early`.

## Residual Gap

`slash_bond` still updates `slashed_amount` without transferring the slashed tokens out to a treasury. Because of that, the invariant above does not hold globally after slashing until slash custody is wired to a real token sink.

## Checks-Effects-Interactions

All custody-changing exits follow checks-effects-interactions:

1. Validate authorization, amount, and balance constraints.
2. Persist the updated `IdentityBond`.
3. Execute the external token transfer(s).

If the token transfer fails, the transaction reverts and the earlier state update is rolled back atomically by Soroban.

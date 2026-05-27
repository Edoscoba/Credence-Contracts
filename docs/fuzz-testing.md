# Fuzz Testing (Bond Operations)

This repository includes a property-based fuzz harness for the `credence_bond` crate. It exercises randomized sequences of bond operations and asserts the core accounting invariants after every transition.

## Where the tests live

- `contracts/credence_bond/src/fuzz/test_bond_fuzz.rs`

## What is covered

The proptest harness generates randomized sequences of `create`, `top_up`, `slash`, and `withdraw` operations with amounts drawn from the full `i128` range.

It validates the following invariants continuously:

- `bonded_amount >= 0`
- `slashed_amount >= 0`
- `slashed_amount <= bonded_amount`
- `available_balance = bonded_amount - slashed_amount >= 0`
- withdrawals never exceed available balance
- bond tier assignment is monotonic with respect to bonded amount

## Why this matters

These invariants protect the core bond accounting logic from overflow, underflow, and invalid sequence interactions. The harness is especially valuable for:

- extreme amounts near `i128::MIN` and `i128::MAX`
- interleaved slash and withdraw operations
- invalid or rejected operations that must preserve state invariants
- tier threshold behavior under changing bond balances

## Running locally

Run the harness directly with:

```bash
cargo test -p credence_bond fuzz::test_bond_fuzz -- --nocapture
```

The `--nocapture` flag ensures proptest prints the failing case details and shrinking output.

## CI integration

The repository CI already runs this harness explicitly in the test job using:

```bash
cargo test -p credence_bond fuzz::test_bond_fuzz -- --nocapture
```

## Interpreting failures

A failing harness prints a minimal reproducible case and seed. To investigate:

1. copy the proptest failure details from the test output
2. rerun the harness locally with the same command
3. inspect the generated operation sequence and the asserted invariant message

## Security notes

This harness encodes arithmetic invariants as assertions rather than application logic. That means a failure points to a violation in the accounting model, not only to a panic.

- The harness protects against negative balances and invalid slashing/withdrawals.
- It also guards tier monotonicity so bond classification does not regress unexpectedly.
- Shrinking produces the smallest counterexample for faster debugging.

# Synthesizer Decision Logic

The synthesizer converts a `TransactionIR` into a `PolicySpec` using a deterministic
decision tree. This document explains every branch.

## Inputs

- `TransactionIR` — produced by the analyzer from Stellar auth entries
- `SynthesisOptions` — user-configurable knobs:
  - `force_custom` (default: false) — skip primitive selection entirely
  - `spending_limit_multiplier` (default: 1.1) — amount ceiling = observed max × this
  - `window_ledgers` (default: 17280 ≈ 1 day) — rolling window size

## Decision Tree

```
1. Flatten auth tree → collect all AuthNodes via BFS
2. Deduplicate signers

IF force_custom=true
  → kind = Custom

ELSE IF signers.len() == 1
       AND all functions ∈ {transfer, transfer_from, approve, burn, burn_from}
       AND token_flows share exactly one token
  → kind = SpendingLimit

ELSE IF signers.len() > 1
       AND all signers are at the same invocation depth
  → kind = SimpleThreshold

ELSE IF signers.len() > 1
       AND signers are at different depths
  → kind = WeightedThreshold

ELSE
  → kind = Custom
```

The `Custom` path is taken whenever:
- Protocol-specific functions appear (claim, deposit, swap, etc.)
- Multi-token flows prevent a single SpendingLimit from applying
- `force_custom=true` was set by the user

## Arg Constraint Extraction

For every `AuthNode`, the synthesizer scans each argument by index:

| Argument type | Condition | Constraint emitted |
|---|---|---|
| `I128` / `U128` | function in `AMOUNT_BEARING_FNS` | `MaxAmount { max: observed × multiplier }` |
| `Address` | not equal to `source_account` | `ExactAddress { address }` |
| `Vec<Address>` | function is a swap function, arg index == 2 | `PathContains { allowed: observed_path }` |
| `U64` | function is a swap function, arg index == 4 | `Deadline` |

## Warnings Emitted Automatically

- `"spending limit derived from a single observed transaction"` — when `token_flows.len() == 1`
- `"OZ smart accounts allow a maximum of 5 policies per context rule"` — when `allowed_contracts.len() > 5`

## Amount Ceiling Calculation

```
ceiling = ceil(max_observed_amount × spending_limit_multiplier)
```

Default multiplier of 1.1 adds a 10% headroom. This is intentionally conservative.
The engineer must actively raise it for transactions that need more room.

## Signer Weight Inference (WeightedThreshold)

Weight is inversely proportional to invocation depth:

```
weight = (max_depth_in_set - node_depth) + 1
```

A signer whose invocation is at depth 0 (root) gets the highest weight.
A signer whose invocation is nested deeper gets lower weight.
This is a heuristic — the user should review weights before deploying.

---
name: oz-policy-builder
version: 0.1.0
description: >
  Synthesize OpenZeppelin smart account policies for Stellar Soroban from observed
  transactions. Use when a user wants to scope what a smart account can do, derive
  minimal access control from a real transaction sequence, generate Rust Policy trait
  code, or test permit/deny behavior before deploying.
tools:
  - observe_transaction
  - observe_simulation
  - synthesize_policy
  - generate_code
  - run_harness
---

# OZ Policy Builder

## What this skill does

Walks through a five-phase workflow to convert a Stellar transaction into a
deployable OpenZeppelin policy contract:

1. **Observe** — record auth entries from a real tx hash or a simulation XDR
2. **Synthesize** — derive the minimal context rule + policy spec
3. **Generate** — emit reviewable Rust code (Cargo.toml + lib.rs + tests.rs)
4. **Simulate** — run permit/deny harness; surface false-permissive violations
5. **Hand off** — provide compile + deploy commands for the user to execute

Deployment is **never automated**. The skill produces reviewable artifacts only.

---

## Workflow

### Phase 1 — Observe

Call `observe_transaction` when the user provides a transaction hash:
```
observe_transaction(tx_hash="abc123...", network="mainnet")
```

Call `observe_simulation` when the user provides raw XDR or wants to test before submitting:
```
observe_simulation(xdr="<base64 xdr>", network="testnet")
```

If multiple representative transactions are provided (e.g., one claim + one deposit),
call `observe_*` for each and note all returned IRs. Merge them mentally: the union
of all `allowed_contracts` and `allowed_calls` across IRs forms the broadest policy.
Ask the user whether to use the union or to pick a specific flow.

### Phase 2 — Synthesize

Call `synthesize_policy` with the IR and a descriptive `context_rule_id`:
```
synthesize_policy(ir=<IR>, context_rule_id="blend_yield_daily")
```

**Before confirming the spec, review reasoning[] with the user:**
- If `token_flows` has only one entry, warn: "This limit is derived from a single observation. Consider recording more transactions to calibrate the ceiling."
- If an amount constraint was derived, ask: "The observed max transfer was X. The policy ceiling is set to X * 1.1. Should I tighten it, raise it, or keep the default?"
- If `primitive_used` is "Custom" because of protocol-specific functions, explain why.

Use `overrides.force_custom=true` if the user wants a fully auditable custom contract
even when a primitive would suffice.

### Phase 3 — Generate

Call `generate_code` with the confirmed spec:
```
generate_code(spec=<spec>, package_name="my-blend-policy")
```

**Always show the user `src/lib.rs` before proceeding.**
Highlight any `warnings[]` from the response.
Do not suggest deployment until the user confirms the code looks correct.

### Phase 4 — Simulate

Call `run_harness` to validate permit/deny behavior:
```
run_harness(spec=<spec>)
```

If `failed > 0`:
- For a deny vector that passed (false permissive): flag it as a security issue and
  re-synthesize with tighter constraints or `force_custom=true`.
- For a permit vector that failed (false restrictive): explain which constraint
  blocked it and offer to widen the relevant ceiling.

### Phase 5 — Deploy (user action only)

Show `next_steps` from `generate_code` output verbatim. Never execute them.
The user reviews, compiles, and deploys. Suggest they coordinate with OpenZeppelin
reviewers before mainnet deployment.

---

## Rules

- Never call `generate_code` without first showing the user the PolicySpec.
- Never auto-deploy. Never suggest `stellar contract deploy` as something you will run.
- Spending limits derived from a single observation always carry a warning. Always surface it.
- Storage keys in custom policies must include (smart_account_address, context_rule_id). If reviewing generated code, check this.
- OZ smart accounts allow a maximum of 5 policies per context rule. Warn if synthesis would exceed this.
- If `allowed_contracts` in the spec is empty, something went wrong — do not proceed.

---

## Worked Examples

### Example 1: Blend Yield Claim

```
User: "I want to delegate my daily Blend yield claim to a bot. Here's a tx where I claimed manually: txhash=abc123"

1. observe_transaction(tx_hash="abc123")
   → IR: auth_roots=[claim(backstop), deposit(pool)], token_flows=[{BLND, backstop→pool, 1500}]
   → protocol: Blend

2. synthesize_policy(ir, context_rule_id="blend_yield_daily")
   → kind: Custom (claim+deposit not covered by spending_limit primitive alone)
   → allowed_contracts: [backstop, pool]
   → allowed_calls: [claim, deposit] with ExactAddress constraints
   → Ask user: "Observed 1500 BLND. Ceiling set to 1650. Ok?"

3. generate_code(spec, package_name="blend-yield-policy")
   → show lib.rs

4. run_harness(spec)
   → permit_exact_claim: PASS
   → deny_wrong_contract: PASS
   → deny_wrong_function: PASS
   → All vectors: PASS

5. Show next_steps. User compiles and deploys.
```

### Example 2: Soroswap Bounded Swap

```
User: "I want to let an agent swap XLM for USDC on Soroswap, max 100 USDC per swap, only that route."

1. observe_simulation(xdr="<soroswap xdr>", network="testnet")
   → IR: swap_exact_tokens_for_tokens(router, path=[USDC,XLM], amount_in=100_0000000)
   → protocol: Soroswap

2. synthesize_policy(ir, context_rule_id="soroswap_xlm_usdc")
   → kind: Custom (path constraint + deadline enforcement)
   → allowed_contracts: [soroswap_router]
   → PathContains: [USDC, XLM]
   → MaxAmount: 110_0000000 (10% headroom)
   → Deadline enforced

3. generate_code(spec, package_name="soroswap-bounded-swap")

4. run_harness(spec) → deny_path_deviation: PASS, deny_amount_2x: PASS

5. Show deploy commands.
```

### Example 3: SEP-41 Subscription

```
User: "I want to allow a subscription contract to pull 10 USDC per month from my smart account."

1. observe_transaction(approve_tx_hash) + observe_transaction(transfer_from_tx_hash)
   → IR union: approve(subscriber, 10_USDC) + transfer_from(subscriber, me, billing, 10_USDC)

2. synthesize_policy(ir, context_rule_id="usdc_subscription_monthly")
   → kind: Custom (call count + spender pinning)
   → WrongSpender, AllowanceTooHigh, AmountExceedsPerCall, WrongBeneficiary constraints
   → window_ledgers: 120960 (≈ 7 days); ask user if monthly cadence preferred

3. generate_code(spec, package_name="usdc-subscription")

4. run_harness(spec) → deny_wrong_spender: PASS, deny_approve_exceeds_ceiling: PASS

5. Show deploy commands.
```

# Wallet Integration

## Overview

The policy builder is a developer tool. Wallet integration means wiring the
record → generate → simulate → install flow into an existing Stellar wallet
that supports OZ smart accounts (C-addresses).

## Prerequisites

The wallet must:
1. Use an OZ smart account contract (implements `__check_auth`)
2. Support calling arbitrary contract functions from the wallet UI
3. Be able to sign and submit Soroban transactions

Compatible wallets as of Q2 2026:
- **Meridian Pay** — passkey-based, OZ smart account, open source
- **Meru Wallet** — non-custodial USDC wallet, Blend integration

## Integration Steps

### Step 1 — Record the transaction

From the wallet's transaction history, extract the transaction hash for the
representative operation the user wants to delegate. Pass it to `observe_transaction`.

```typescript
const { ir } = await mcpClient.callTool("observe_transaction", {
  tx_hash: txHashFromWallet,
  network: "mainnet",
});
```

### Step 2 — Synthesize and review

Call `synthesize_policy` and present the resulting `PolicySpec` to the user in
the wallet UI. Show:
- Which contracts are being whitelisted
- What the spending ceiling is (if applicable)
- Any warnings (single observation, etc.)

The user must confirm before proceeding.

### Step 3 — Generate and compile

Call `generate_code` to get the Rust source files. For a wallet integration,
you have two options:

**Option A (recommended): Use a pre-compiled reference policy**

For common patterns (Blend yield claim, Soroswap bounded swap, SEP-41 subscription),
use the reference policy contracts from `packages/policies/`. These are pre-audited.
Configure them via their `install()` parameters derived from the `PolicySpec`.

**Option B: Compile generated code**

For custom patterns, the generated Rust code must be compiled to WASM before
deployment. This requires a build server or a CI pipeline — it cannot happen
in the browser or mobile app.

### Step 4 — Deploy the policy contract

```bash
stellar contract deploy \
  --wasm <policy.wasm> \
  --source <wallet_keypair> \
  --network mainnet
```

The wallet submits this transaction. The resulting contract address is the
`policy_address`.

### Step 5 — Install the policy into the context rule

Call the smart account's `add_context_rule` (or equivalent OZ function) to
attach the policy to a context rule scoped to the delegated operation.

```bash
stellar contract invoke \
  --id <smart_account_address> \
  --source <wallet_keypair> \
  --network mainnet \
  -- add_context_rule \
     --context <serialized_context> \
     --policy <policy_address>
```

The wallet presents this as a one-time setup transaction.

### Step 6 — Verify

After installation, use the simulation harness (`run_harness`) to confirm that:
- The delegated agent can perform the permitted operation (permit vector passes)
- The agent cannot deviate (deny vectors pass)

## Security Notes

- Never auto-install without explicit user confirmation.
- Show the full `PolicySpec` to the user before presenting the install transaction.
- Reference policies are audited; generated custom policies are not — label them clearly.
- The policy deployment and installation are two separate transactions. Both require user signature.

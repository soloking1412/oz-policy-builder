# Example: SEP-41 Subscription

Demonstrates how to derive and generate a policy for a recurring payment subscription —
the `approve` + `transfer_from` pattern where an allowance is set for a subscriber contract
which then pulls payments to a fixed beneficiary on a schedule.

## What this policy allows

- `approve` on the configured token: only to the subscriber address, amount ≤ allowance ceiling
- `transfer_from` on the configured token: per-call amount ≤ `max_per_call`, `to` must be the
  beneficiary, call count ≤ `max_calls_per_window` within the rolling window

## Prerequisites

- Rust toolchain with `wasm32v1-none` target
- `stellar` CLI
- A SEP-41 token contract and a subscriber contract on testnet

## Record a real transaction

Run from the repository root:

```bash
export STELLAR_RPC_URL=https://soroban-testnet.stellar.org
tsx examples/record.ts sep41-subscription <subscription_setup_tx_hash>
```

## Synthesize and generate via MCP

```typescript
const { ir } = await mcpClient.callTool("observe_transaction", {
  tx_hash: "<subscription_tx_hash>",
  network: "testnet",
});

const { spec } = await mcpClient.callTool("synthesize_policy", {
  ir,
  context_rule_id: "usdc_monthly_sub",
  overrides: { force_custom: true },
});

const { files } = await mcpClient.callTool("generate_code", {
  spec,
  package_name: "sep41-subscription-policy",
});
```

## Key constraints synthesized

| Function | Arg | Index | Constraint |
|---|---|---|---|
| `approve` | spender | 1 | `ExactAddress` (subscriber only) |
| `approve` | amount | 2 | `MaxAmount` (allowance ceiling) |
| `transfer_from` | to | 2 | `ExactAddress` (beneficiary only) |
| `transfer_from` | amount | 3 | `MaxAmount` (per-call limit) |

Call count per window is enforced by the policy's rolling window accumulator.

## Reference policy

See `packages/policies/sep41-subscription` for the production-ready contract.
The `Config` struct maps directly to the synthesized `PolicySpec` parameters.

# Example: Soroswap Bounded Swap

Demonstrates how to derive and generate a policy for a Soroswap DEX swap —
specifically constraining `swap_exact_tokens_for_tokens` to a fixed maximum input
amount, an approved token path, and a future deadline.

## What this policy allows

- `swap_exact_tokens_for_tokens` and `swap_tokens_for_exact_tokens` on the configured router
- `amount_in` ≤ `max_amount_in` per individual call
- Rolling window: total `amount_in` across all calls within `window_ledgers` ≤ `max_amount_in`
- Swap path may only contain tokens in `allowed_tokens`
- `deadline` argument must be ≥ current ledger timestamp

## Prerequisites

- Rust toolchain with `wasm32v1-none` target
- `stellar` CLI
- A Stellar testnet or mainnet account with USDC or XLM

## Record a real transaction

Run from the repository root:

```bash
export STELLAR_RPC_URL=https://soroban-testnet.stellar.org
tsx examples/record.ts soroswap-bounded <swap_tx_hash>
```

## Synthesize and generate via MCP

```typescript
const { ir } = await mcpClient.callTool("observe_transaction", {
  tx_hash: "<swap_tx_hash>",
  network: "testnet",
});

const { spec } = await mcpClient.callTool("synthesize_policy", {
  ir,
  context_rule_id: "soroswap_daily",
  overrides: { spending_limit_multiplier: 1.2 },
});

const { files } = await mcpClient.callTool("generate_code", {
  spec,
  package_name: "soroswap-bounded-policy",
});
```

## Key constraints synthesized

| Arg | Index | Constraint |
|---|---|---|
| `amount_in` | 0 | `MaxAmount` (observed × multiplier) |
| `path` | 2 | `PathContains` (only observed tokens allowed) |
| `deadline` | 4 | `Deadline` (must be > current timestamp) |

## Reference policy

See `packages/policies/soroswap-bounded` for the production-ready contract.

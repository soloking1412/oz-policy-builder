# Example: Blend Yield Claim

Demonstrates how to derive and generate a policy for a Blend Finance yield harvesting flow —
specifically the `claim()` + `deposit()` pattern where earned BLND emissions are claimed from
the backstop contract and re-deposited into a Blend lending pool.

## What this policy allows

- `claim` on the configured backstop contract
- `deposit` on the configured pool contract
- Up to `max_per_window` calls per `window_ledgers` ledger window
- No other contracts or functions

## Prerequisites

- Rust toolchain with `wasm32v1-none` target (`rustup target add wasm32v1-none`)
- `stellar` CLI for deployment
- A Stellar testnet or mainnet account

## Record a real transaction

Run from the repository root:

```bash
export STELLAR_RPC_URL=https://soroban-testnet.stellar.org
tsx examples/record.ts blend-yield-claim <transaction_hash>
```

This fetches the transaction from the RPC, extracts auth entries and diagnostic events,
and writes them to `fixtures/sim_response.json`.

## Synthesize and generate via MCP

```typescript
// 1. Observe
const { ir } = await mcpClient.callTool("observe_transaction", {
  tx_hash: "<tx_hash>",
  network: "testnet",
});

// 2. Synthesize
const { spec } = await mcpClient.callTool("synthesize_policy", {
  ir,
  context_rule_id: "blend_yield_daily",
});

// 3. Generate
const { files } = await mcpClient.callTool("generate_code", {
  spec,
  package_name: "blend-yield-policy",
});

// 4. Run harness
const report = await mcpClient.callTool("run_harness", { spec });
console.log(`${report.passed}/${report.results.length} vectors passed`);
```

## Reference policy

The `packages/policies/blend-yield-claim` contract is a pre-built, auditable implementation
of exactly this pattern. For production use, prefer deploying that contract and configuring it
via `install(account, rule_id, config)` with the addresses derived from synthesis.

## Fixture

`fixtures/sim_response.json` contains a synthetic stand-in representing the expected auth
structure. Replace it with real testnet data by running `record.ts` against a live transaction.

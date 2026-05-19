# OZ Policy Builder

Tooling for deriving OpenZeppelin smart account policies on Stellar from transactions
you've already run.

OpenZeppelin's smart accounts on Soroban let you attach *policies* to a *context rule* —
small contracts that decide whether a given call is allowed. They're a good way to hand a
key to an agent or a bot without handing over the whole account. The hard part is writing
the policy contract: you have to know exactly which contracts, functions, and argument
bounds to allow, and get it tight enough to be safe but loose enough to still work.

This project takes a different route. You point it at a real transaction — one you signed
yourself, or one you simulated — and it works backwards. It reads the authorization tree,
figures out the narrowest policy that would have permitted exactly that call, and emits the
Rust contract for you to review. Nothing is deployed automatically. The output is source
code and a test report; what you do with them is up to you.

## How it works

The pipeline has four stages, plus a deploy step that's always yours to run:

1. **Observe** — fetch a transaction by hash, or simulate a transaction envelope. Either
   way you get a `TransactionIR`: a structured view of the auth entries, token flows, and
   any protocols that were recognised (Blend, Soroswap, SEP-41).
2. **Synthesize** — turn the IR into a `PolicySpec`. This is where the decision logic
   lives: a single signer moving one token becomes a spending-limit policy; several signers
   become a threshold policy; anything with protocol-specific calls or argument constraints
   becomes a custom contract. Every choice is recorded in `reasoning[]` so you can see why.
3. **Generate** — emit the actual crate: `Cargo.toml`, `src/lib.rs`, `src/tests.rs`. For
   the three primitive kinds this is a thin wrapper over OZ's own policy contracts; for a
   custom policy it's a complete `#![no_std]` contract.
4. **Harness** — run permit/deny vectors against the spec. They're built automatically: the
   observed call should pass, a doubled amount should fail, a swapped-in contract address
   should fail, and so on.

Deploying is step five, and it's yours. The toolkit prints the `stellar contract` commands
and stops there — there is no deploy command in the CLI and no deploy tool in the server.

## Layout

```
packages/
  core/        Rust synthesizer engine — analyzer, synthesizer, codegen, harness
  cli/         oz-policy-core: a JSON-over-stdio bridge to the core
  mcp-server/  TypeScript MCP server exposing the pipeline as five tools
  policies/    three hand-written reference policy contracts
examples/      one walkthrough per reference policy, plus record.ts to grab fixtures
tests/         integration walkthroughs for the full Blend / Soroswap / SEP-41 flows
docs/          synthesizer decision logic, how to add a primitive, wallet integration
SKILL.md       Claude skill definition that drives the five tools
```

One design choice worth calling out: all XDR parsing happens in TypeScript, using
`@stellar/stellar-sdk`. The Rust core never touches XDR — it takes plain JSON. That keeps
the core simple to test and puts the version-sensitive part of the stack where the mature
library already is.

## Building

You'll need a Rust toolchain and Node 18+ with pnpm.

```
cargo build --workspace
cargo test --workspace
```

That builds the core and the CLI and runs the full Rust suite — unit tests plus the
integration walkthroughs under `tests/`.

The MCP server is a separate pnpm package:

```
cd packages/mcp-server
pnpm install
pnpm build      # tsc
pnpm test       # vitest
```

The server shells out to the `oz-policy-core` binary. After `cargo build --release` it
lives at `target/release/oz-policy-core` and the server finds it relative to itself; set
`OZ_POLICY_CORE_BIN` if you keep it somewhere else.

## Reference policies

`packages/policies/` has three contracts written by hand, not generated:

- **blend-yield-claim** — allows `claim` on a Blend backstop and `deposit` on a pool, with
  a per-window call cap.
- **soroswap-bounded** — allows swaps on one router, caps `amount_in`, pins the token path,
  and enforces the deadline.
- **sep41-subscription** — allows `approve` / `transfer_from` for a recurring pull payment,
  with the spender and beneficiary pinned and a per-window call count.

They serve as the targets the synthesizer aims at, and as something safer to deploy than
freshly generated code. Build them to wasm with:

```
cargo build -p blend-yield-claim-policy -p soroswap-bounded-policy -p sep41-subscription-policy \
  --target wasm32v1-none --profile contract
```

That target needs `rustup target add wasm32v1-none`.

## End to end

`scripts/e2e.sh` runs the whole thing — builds the workspace, runs the tests, pipes a
synthetic Soroswap transaction through synthesize and generate, and builds the reference
contracts:

```
bash scripts/e2e.sh
```

## A few things to keep in mind

- **Generated code is not audited.** The three policies in `packages/policies/` are written
  and reviewed by hand; anything that comes out of `generate` is a starting point. Read the
  generated `src/lib.rs` line by line before it goes near a network.
- **A limit from one transaction is a guess.** Synthesize a spending limit from a single
  observed transfer and the synthesizer says so in `warnings[]`. Record a few representative
  transactions if you can.
- **Deployment is never automated.** Signing and submitting is always a separate, deliberate
  step you take with your own keys.
- **Five policies per context rule** is an OZ limit; synthesis warns if a spec would exceed it.

## Docs

- `docs/synthesizer-decisions.md` — every branch of the decision tree, and how amounts and
  weights are derived.
- `docs/extending-primitives.md` — how to add a new policy primitive when OZ ships one.
- `docs/wallet-integration.md` — wiring the record → generate → install flow into a wallet.

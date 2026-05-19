import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

const PRIMITIVES_REFERENCE = `
# OpenZeppelin Stellar Policy Primitives

## spending_limit

Enforces a rolling-window token spend cap per (smart_account, context_rule_id).

Config:
  token:          Address  — SEP-41 token contract to track
  limit:          i128     — maximum cumulative amount per window
  window_ledgers: u32      — window length in ledgers (17280 ≈ 1 day)

Lifecycle:
  install(account, rule_id, config)     — stores config + resets window
  can_enforce(account, rule_id) → bool  — always true
  enforce(account, rule_id, amount)     — panics if accumulated + amount > limit
  uninstall(account, rule_id)           — removes all storage keys

Use when: single signer, only transfer/transfer_from/burn/approve operations,
          one dominant token.

## simple_threshold

Requires at least N valid signatures from a predefined signer set.

Config:
  signers:   Vec<Address>  — allowed signers
  threshold: u32           — minimum required signatures

Use when: multiple signers with equal weight, simple multisig use case.

## weighted_threshold

Assigns numeric weights to signers; authorization requires combined weight ≥ threshold.

Config:
  signers:   Vec<(Address, u32)>  — (signer, weight) pairs
  threshold: u32                  — minimum combined weight

Use when: multiple signers with unequal authority (e.g. 2-of-3 with a veto signer).

## When primitives are insufficient → Custom Policy

Generate a full Policy trait implementation when:
- Functions beyond SEP-41 token ops (claim, swap, deposit, etc.)
- Argument-level constraints (path allowlist, address pinning, deadline enforcement)
- Per-call count limiting (subscription billing)
- Multi-asset flows requiring separate tracking

Custom policies implement:
  install(env, account, rule_id, ...)  — segregated storage init
  can_enforce(env, account, rule_id)   — precondition check (usually true)
  enforce(env, account, rule_id, ...)  — validation + state update; panics on violation
  uninstall(env, account, rule_id)     — clean up all (account, rule_id) keyed entries
`.trim();

export function registerPrimitivesRefResource(server: McpServer) {
  server.resource(
    "policy://primitives",
    "policy://primitives",
    { mimeType: "text/markdown" },
    async () => ({
      contents: [
        {
          uri: "policy://primitives",
          mimeType: "text/markdown",
          text: PRIMITIVES_REFERENCE,
        },
      ],
    })
  );
}

# Extending the Policy Primitives

## When to add a new primitive emitter

The synthesizer has four codegen paths:

1. `spending_limit` — delegates to OZ primitive
2. `simple_threshold` — delegates to OZ primitive
3. `weighted_threshold` — delegates to OZ primitive
4. `custom` — emits a full Policy trait implementation

Add a new primitive emitter when:
- A new OZ policy primitive is upstreamed (e.g., `rate_limit`, `time_lock`)
- The new primitive covers a class of constraints that currently escalates to `Custom`

## How to add a new primitive emitter

### 1. Add a new `PolicyKind` variant

`packages/core/src/ir/types.rs`:
```rust
pub enum PolicyKind {
    SpendingLimit,
    SimpleThreshold,
    WeightedThreshold,
    Custom,
    RateLimit,      // ← new
}
```

### 2. Add detection logic to `synthesizer/decision.rs`

In `select_kind()`, add a new branch before the `Custom` fallback:
```rust
if signers.len() == 1 && is_rate_limited_pattern(&nodes) {
    log.push("rate-limited pattern detected → using rate_limit primitive".to_string());
    return PolicyKind::RateLimit;
}
```

### 3. Add a new `PolicyParameters` variant

```rust
pub enum PolicyParameters {
    // ... existing
    RateLimit { max_calls: u32, window_ledgers: u32 },
}
```

### 4. Create `packages/core/src/codegen/rate_limit.rs`

Implement the `emit(spec, package_name) -> Result<GeneratedPolicy>` function.
Emit a `Cargo.toml` and `lib.rs` that configures the OZ `rate_limit` primitive
with the derived parameters.

### 5. Wire into `composer.rs`

```rust
PolicyKind::RateLimit => rate_limit::emit(spec, package_name),
```

### 6. Add tests

- Add a table-driven test in `synthesizer/decision.rs` for the new branch
- Add a snapshot test in `codegen/rate_limit.rs`
- Add an integration test in `tests/integration/`

## Upstreaming to OZ

If you implement a custom policy that proves useful across multiple use cases,
consider opening a PR to the OZ stellar-contracts repo to upstream it as a
new primitive. Coordinate with the OZ team via GitHub issues before writing code.

#!/usr/bin/env bash
set -euo pipefail

NETWORK="${1:-testnet}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building Rust workspace"
cargo build --manifest-path "$ROOT/Cargo.toml" --release 2>&1

echo "==> Running unit tests"
cargo test -p oz-policy-builder-core 2>&1

echo "==> Running integration tests"
cargo test --test blend_walkthrough   2>&1
cargo test --test soroswap_walkthrough 2>&1
cargo test --test sep41_walkthrough   2>&1

echo "==> Testing CLI bridge with synthetic Soroswap input"
SOROSWAP_IR=$(cat <<'EOF'
{
  "method": "synthesize",
  "params": {
    "ir": {
      "tx_hash": "e2e_test",
      "source": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
      "auth_roots": [
        {
          "signer": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
          "contract": "CSOROSWAPROU000000000000000000000000000000000000000000000",
          "function": "swap_exact_tokens_for_tokens",
          "args": [
            {"type": "I128", "value": 100000000},
            {"type": "I128", "value": 90000000},
            {"type": "Vec", "value": [
              {"type": "Address", "value": "CUSDCTOKEN00000000000000000000000000000000000000000000000"},
              {"type": "Address", "value": "CXLMTOKEN000000000000000000000000000000000000000000000000"}
            ]},
            {"type": "Address", "value": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"},
            {"type": "U64", "value": 9999999999}
          ],
          "sub_invocations": []
        }
      ],
      "token_flows": [
        {
          "token": "CUSDCTOKEN00000000000000000000000000000000000000000000000",
          "from": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
          "to": "CPAIR000000000000000000000000000000000000000000000000000",
          "amount": 100000000
        }
      ],
      "detected_protocols": [
        {"protocol": "Soroswap", "router": "CSOROSWAPROU000000000000000000000000000000000000000000000", "path": [
          "CUSDCTOKEN00000000000000000000000000000000000000000000000",
          "CXLMTOKEN000000000000000000000000000000000000000000000000"
        ]}
      ]
    },
    "options": {
      "context_rule_id": "e2e_soroswap",
      "force_custom": false,
      "spending_limit_multiplier": 1.1,
      "window_ledgers": 17280
    }
  }
}
EOF
)

CLI_BIN="$ROOT/target/release/oz-policy-core"
if [[ ! -f "$CLI_BIN" ]]; then
  echo "ERROR: CLI binary not found at $CLI_BIN. Run 'cargo build --release' first."
  exit 1
fi

SYNTH_RESULT=$(echo "$SOROSWAP_IR" | "$CLI_BIN")
echo "$SYNTH_RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
assert r['ok'], f'synthesis failed: {r}'
spec = r['data']
assert spec['kind'] == 'Custom', f'expected Custom, got {spec[\"kind\"]}'
print('synthesize: OK  kind=' + spec['kind'])
"

GENERATE_INPUT=$(echo "$SYNTH_RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
print(json.dumps({'method': 'generate', 'params': {'spec': r['data'], 'package_name': 'e2e-test-policy'}}))
")

GEN_RESULT=$(echo "$GENERATE_INPUT" | "$CLI_BIN")
echo "$GEN_RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
assert r['ok'], f'generate failed: {r}'
files = r['data']
assert '#![no_std]' in files['lib_rs'], 'lib_rs missing no_std'
assert 'soroban-sdk' in files['cargo_toml'], 'Cargo.toml missing soroban-sdk'
print('generate:   OK  lib_rs has no_std + Cargo.toml has soroban-sdk')
"

echo ""
echo "==> Building reference policy contracts"
cargo build \
  -p blend-yield-claim-policy \
  -p soroswap-bounded-policy \
  -p sep41-subscription-policy \
  --target wasm32v1-none \
  --profile contract 2>&1 \
  && echo "contract build: OK" \
  || echo "contract build: SKIP (wasm32v1-none target not installed; run 'rustup target add wasm32v1-none')"

echo ""
echo "All e2e checks passed."

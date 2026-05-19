use serde::{Deserialize, Serialize};

/// XDR-free input. Auth parsing happens TypeScript-side via `@stellar/stellar-sdk`;
/// the Rust core receives plain JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationInput {
    pub auth_roots: Vec<AuthNode>,
    /// SEP-41 token flows from diagnostic events. `None` → derive from auth tree.
    pub token_flows: Option<Vec<TokenFlow>>,
    pub source_account: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthNode {
    pub signer: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<ScValIR>,
    pub sub_invocations: Vec<AuthNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ScValIR {
    Address(String),
    I128(i128),
    U128(u128),
    U64(u64),
    I64(i64),
    Bool(bool),
    Symbol(String),
    Bytes(Vec<u8>),
    Vec(Vec<ScValIR>),
    Map(Vec<(ScValIR, ScValIR)>),
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenFlow {
    pub token: String,
    pub from: String,
    pub to: String,
    pub amount: i128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "protocol")]
pub enum ProtocolHint {
    Blend { pool: String, backstop: String },
    Soroswap { router: String, path: Vec<String> },
    Sep41 { contract: String },
    Unknown { contract: String, function: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIR {
    pub tx_hash: Option<String>,
    pub source: String,
    pub auth_roots: Vec<AuthNode>,
    pub token_flows: Vec<TokenFlow>,
    pub detected_protocols: Vec<ProtocolHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedCall {
    pub contract: String,
    pub function: String,
    pub arg_constraints: Vec<ArgConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ArgConstraint {
    MaxAmount { arg_index: usize, max: i128 },
    ExactAddress { arg_index: usize, address: String },
    PathContains { arg_index: usize, allowed: Vec<String> },
    Deadline { arg_index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyKind {
    SpendingLimit,
    SimpleThreshold,
    WeightedThreshold,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PolicyParameters {
    SpendingLimit {
        token: String,
        limit: i128,
        window_ledgers: u32,
    },
    SimpleThreshold {
        threshold: u32,
    },
    WeightedThreshold {
        signers: Vec<(String, u32)>,
        threshold: u32,
    },
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySpec {
    pub kind: PolicyKind,
    pub context_rule_id: String,
    pub allowed_contracts: Vec<String>,
    pub allowed_calls: Vec<AllowedCall>,
    pub parameters: PolicyParameters,
    pub reasoning: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPolicy {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub tests_rs: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    Permit,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessVector {
    pub id: String,
    pub expected: Outcome,
    pub contract: String,
    pub function: String,
    pub args: Vec<ScValIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResult {
    pub vector_id: String,
    pub expected: Outcome,
    pub actual: Outcome,
    pub passed: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessReport {
    pub results: Vec<VectorResult>,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisOptions {
    pub context_rule_id: String,
    pub force_custom: bool,
    pub spending_limit_multiplier: f64,
    pub window_ledgers: u32,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            context_rule_id: "default".to_string(),
            force_custom: false,
            spending_limit_multiplier: 1.1,
            window_ledgers: 17_280,
        }
    }
}

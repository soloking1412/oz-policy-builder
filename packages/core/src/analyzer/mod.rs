pub mod invocation;
pub mod token;

use crate::error::Result;
use crate::ir::types::{SimulationInput, TransactionIR};

pub fn analyze(input: SimulationInput) -> Result<TransactionIR> {
    let token_flows = if let Some(flows) = input.token_flows {
        flows
    } else {
        token::extract_from_auth(&input.auth_roots)
    };

    let detected_protocols = invocation::detect(&input.auth_roots);

    Ok(TransactionIR {
        tx_hash: input.tx_hash,
        source: input.source_account,
        auth_roots: input.auth_roots,
        token_flows,
        detected_protocols,
    })
}

pub mod analyzer;
pub mod codegen;
pub mod error;
pub mod harness;
pub mod ir;
pub mod synthesizer;

use error::Result;
use ir::types::{
    GeneratedPolicy, HarnessReport, HarnessVector, PolicySpec, SimulationInput,
    SynthesisOptions, TransactionIR,
};

pub fn analyze(input: SimulationInput) -> Result<TransactionIR> {
    analyzer::analyze(input)
}

pub fn synthesize(ir: TransactionIR, opts: SynthesisOptions) -> Result<PolicySpec> {
    synthesizer::synthesize(ir, opts)
}

pub fn generate(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    codegen::generate(spec, package_name)
}

pub fn run_harness(
    spec: &PolicySpec,
    extra_permit: Vec<HarnessVector>,
    extra_deny: Vec<HarnessVector>,
) -> Result<HarnessReport> {
    harness::run(spec, extra_permit, extra_deny)
}

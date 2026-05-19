use crate::error::Result;
use crate::ir::types::{GeneratedPolicy, PolicyKind, PolicySpec};
use crate::codegen::{custom, simple_threshold, spending_limit, weighted_threshold};

pub fn generate(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    match spec.kind {
        PolicyKind::SpendingLimit => spending_limit::emit(spec, package_name),
        PolicyKind::SimpleThreshold => simple_threshold::emit(spec, package_name),
        PolicyKind::WeightedThreshold => weighted_threshold::emit(spec, package_name),
        PolicyKind::Custom => custom::emit(spec, package_name),
    }
}

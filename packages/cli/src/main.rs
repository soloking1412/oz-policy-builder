use std::io::{self, BufRead, Write};

use oz_policy_builder_core::{
    self as core,
    ir::types::{HarnessVector, PolicySpec, SimulationInput, SynthesisOptions},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
enum Request {
    Analyze { params: AnalyzeParams },
    Synthesize { params: SynthesizeParams },
    Generate { params: GenerateParams },
    RunHarness { params: HarnessParams },
}

#[derive(Deserialize)]
struct AnalyzeParams {
    input: SimulationInput,
}

#[derive(Deserialize)]
struct SynthesizeParams {
    ir: oz_policy_builder_core::ir::types::TransactionIR,
    options: SynthesisOptions,
}

#[derive(Deserialize)]
struct GenerateParams {
    spec: PolicySpec,
    package_name: String,
}

#[derive(Deserialize)]
struct HarnessParams {
    spec: PolicySpec,
    #[serde(default)]
    extra_permit: Vec<HarnessVector>,
    #[serde(default)]
    extra_deny: Vec<HarnessVector>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Response<T: Serialize> {
    Ok { ok: bool, data: T },
    Err { ok: bool, error: String, code: String },
}

fn ok<T: Serialize>(data: T) -> Response<T> {
    Response::Ok { ok: true, data }
}

fn err_resp<T: Serialize>(msg: String, code: &str) -> Response<T> {
    Response::Err { ok: false, error: msg, code: code.to_string() }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let response_json = dispatch(&line);
        writeln!(out, "{response_json}").expect("write failed");
        out.flush().expect("flush failed");
    }
}

fn dispatch(line: &str) -> String {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            let resp: Response<()> = err_resp(e.to_string(), "PARSE_ERROR");
            return serde_json::to_string(&resp).unwrap();
        }
    };

    match req {
        Request::Analyze { params } => match core::analyze(params.input) {
            Ok(ir) => serde_json::to_string(&ok(ir)).unwrap(),
            Err(e) => serde_json::to_string(&err_resp::<()>(e.to_string(), "ANALYZE_ERROR")).unwrap(),
        },
        Request::Synthesize { params } => match core::synthesize(params.ir, params.options) {
            Ok(spec) => serde_json::to_string(&ok(spec)).unwrap(),
            Err(e) => serde_json::to_string(&err_resp::<()>(e.to_string(), "SYNTHESIZE_ERROR")).unwrap(),
        },
        Request::Generate { params } => match core::generate(&params.spec, &params.package_name) {
            Ok(policy) => serde_json::to_string(&ok(policy)).unwrap(),
            Err(e) => serde_json::to_string(&err_resp::<()>(e.to_string(), "GENERATE_ERROR")).unwrap(),
        },
        Request::RunHarness { params } => {
            match core::run_harness(&params.spec, params.extra_permit, params.extra_deny) {
                Ok(report) => serde_json::to_string(&ok(report)).unwrap(),
                Err(e) => serde_json::to_string(&err_resp::<()>(e.to_string(), "HARNESS_ERROR")).unwrap(),
            }
        }
    }
}

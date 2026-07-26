use std::time::Instant;

use aegis_common::types::{Decision, PolicyContext, PolicyResult};
use tracing::debug;
use wasmtime::component::Linker;
use wasmtime::{Engine, Module, Store};

pub struct WasmPolicyEngine {
    engine: Engine,
    linker: wasmtime::component::Linker<()>,
    module: Module,
}

impl WasmPolicyEngine {
    pub fn new(wasm_bytes: &[u8]) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;

        Ok(Self {
            engine,
            linker,
            module,
        })
    }

    pub fn evaluate(&self, ctx: &PolicyContext) -> anyhow::Result<PolicyResult> {
        let start = Instant::now();
        let mut store = Store::new(&self.engine, ());

        let instance = self.linker.instantiate(&mut store, &self.module)?;

        // Serialize policy context to JSON and pass to WASM
        let ctx_json = serde_json::to_string(ctx)?;
        let ctx_bytes = ctx_json.as_bytes();

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("WASM module has no memory export"))?;

        // Allocate memory in WASM and write context bytes
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, "alloc")?;
        let ptr = alloc.call(&mut store, ctx_bytes.len() as i32)?;

        memory.write(&mut store, ptr as usize, ctx_bytes)?;

        // Call evaluate function with pointer and length
        let evaluate = instance.get_typed_func::<(i32, i32), i32>(&mut store, "aegis/evaluate")?;
        let result = evaluate.call(&mut store, (ptr, ctx_bytes.len() as i32))?;

        let eval_time_ns = start.elapsed().as_nanos() as i64;

        match result {
            1 => {
                debug!("WASM policy: ALLOW");
                Ok(PolicyResult {
                    decision: Decision::Allow,
                    reason: "WASM: allowed".into(),
                    violated_policies: vec![],
                    evaluation_time_ns: eval_time_ns,
                })
            }
            0 => {
                debug!("WASM policy: DENY");
                Ok(PolicyResult {
                    decision: Decision::Deny,
                    reason: "WASM: denied".into(),
                    violated_policies: vec!["wasm-deny".into()],
                    evaluation_time_ns: eval_time_ns,
                })
            }
            v => {
                debug!("WASM policy: unknown result {v}");
                Ok(PolicyResult {
                    decision: Decision::Allow,
                    reason: format!("WASM: unknown result {v}"),
                    violated_policies: vec![],
                    evaluation_time_ns: eval_time_ns,
                })
            }
        }
    }
}

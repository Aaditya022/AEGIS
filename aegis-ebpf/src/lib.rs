use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

mod events;
mod maps;
mod twoplane;

pub use events::*;
pub use maps::*;
pub use twoplane::*;

pub struct EbpfRuntime {
    bpf: Option<aya::Bpf>,
    event_tx: mpsc::Sender<AegisEvent>,
    event_rx: Arc<RwLock<mpsc::Receiver<AegisEvent>>>,
    two_plane: TwoPlaneVerifier,
    monitored_pids: Arc<RwLock<Vec<u32>>>,
    simulated: bool,
}

impl EbpfRuntime {
    pub async fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(16384);

        Ok(Self {
            bpf: None,
            event_tx: tx,
            event_rx: Arc::new(RwLock::new(rx)),
            two_plane: TwoPlaneVerifier::new(),
            monitored_pids: Arc::new(RwLock::new(Vec::new())),
            simulated: true,
        })
    }

    pub async fn load_and_attach(&mut self) -> anyhow::Result<()> {
        // Try loading each eBPF program
        let bpf_programs = [
            (
                "syscall_monitor",
                aya::include_bytes_aligned!("bpf/syscall_monitor.bpf.o") as &[u8],
            ),
            (
                "tcp_monitor",
                aya::include_bytes_aligned!("bpf/tcp_monitor.bpf.o") as &[u8],
            ),
            (
                "file_access",
                aya::include_bytes_aligned!("bpf/file_access.bpf.o") as &[u8],
            ),
            (
                "credential_monitor",
                aya::include_bytes_aligned!("bpf/credential_monitor.bpf.o") as &[u8],
            ),
        ];

        let mut loaded = Vec::new();
        for (name, bytes) in &bpf_programs {
            match aya::BpfLoader::new().load(bytes) {
                Ok(bpf) => {
                    info!("Loaded eBPF program: {name}");
                    loaded.push((*name, bpf));
                }
                Err(e) => {
                    warn!(program = %name, error = %e, "Failed to load eBPF program, using simulation");
                }
            }
        }

        if loaded.is_empty() {
            info!("No eBPF programs loaded, operating in simulated mode");
            self.simulated = true;
            return Ok(());
        }

        // Attach loaded programs
        for (name, bpf) in &mut loaded {
            let pnames: Vec<String> = bpf.programs().map(|(n, _)| n.to_string()).collect();
            for pname in &pnames {
                if let Some(program) = bpf.program_mut(pname) {
                    if let Ok(tp) =
                        std::convert::TryInto::<&mut aya::programs::TracePoint>::try_into(program)
                    {
                        if let Err(e) = tp.load() {
                            warn!(program = %pname, error = %e, "Failed to load tracepoint");
                        }
                    }
                }
            }
        }

        if let Some((_, first)) = loaded.into_iter().next() {
            self.bpf = Some(first);
            self.simulated = false;
        }

        self.start_event_processor();
        Ok(())
    }

    fn start_event_processor(&self) {
        let two_plane = self.two_plane.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                two_plane.tick();
            }
        });
    }

    pub async fn monitor_pid(&mut self, pid: u32) {
        let mut pids = self.monitored_pids.write().await;
        if !pids.contains(&pid) {
            pids.push(pid);
            debug!(pid, "Now monitoring agent PID");
        }

        if !self.simulated {
            if let Some(ref mut bpf) = self.bpf {
                if let Some(map) = bpf.map_mut("aegis_allowed_pids") {
                    if let Ok(mut hmap) = aya::maps::HashMap::<_, u32, u32>::try_from(map) {
                        let _ = hmap.insert(pid, 1, 0);
                    }
                }
            }
        }
    }

    pub async fn stop_monitoring(&mut self, pid: u32) {
        let mut pids = self.monitored_pids.write().await;
        pids.retain(|p| *p != pid);

        if !self.simulated {
            if let Some(ref mut bpf) = self.bpf {
                if let Some(map) = bpf.map_mut("aegis_allowed_pids") {
                    if let Ok(mut hmap) = aya::maps::HashMap::<_, u32, u32>::try_from(map) {
                        let _ = hmap.remove(&pid);
                    }
                }
            }
        }
    }

    pub fn two_plane_verifier(&self) -> &TwoPlaneVerifier {
        &self.two_plane
    }

    pub fn is_simulated(&self) -> bool {
        self.simulated
    }

    pub async fn get_event_count(&self) -> u64 {
        self.two_plane.total_events()
    }

    pub async fn get_violation_count(&self) -> u64 {
        self.two_plane.total_violations()
    }
}

impl Drop for EbpfRuntime {
    fn drop(&mut self) {
        info!("Cleaning up eBPF programs");
    }
}

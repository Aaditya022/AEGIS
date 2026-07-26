use aya::maps::{HashMap, PerCpuArray};
use aya::Bpf;
use tracing::debug;

/// Helper functions for interacting with eBPF maps from userspace

/// Add a PID to the allowed_pids map
pub fn add_pid_to_monitor(bpf: &Bpf, pid: u32) -> Result<(), anyhow::Error> {
    if let Some(map) = bpf.map_mut("aegis_allowed_pids") {
        if let Ok(mut hmap) = HashMap::<_, u32, u32>::try_from(map) {
            hmap.insert(pid, 1, 0)?;
            debug!(pid, "Added PID to eBPF monitor");
        }
    }
    Ok(())
}

/// Remove a PID from the allowed_pids map
pub fn remove_pid_from_monitor(bpf: &Bpf, pid: u32) -> Result<(), anyhow::Error> {
    if let Some(map) = bpf.map_mut("aegis_allowed_pids") {
        if let Ok(mut hmap) = HashMap::<_, u32, u32>::try_from(map) {
            hmap.remove(&pid)?;
            debug!(pid, "Removed PID from eBPF monitor");
        }
    }
    Ok(())
}

/// Add a sensitive path pattern to the eBPF map
pub fn add_sensitive_path(bpf: &Bpf, pattern: &str) -> Result<(), anyhow::Error> {
    let hash = calculate_fnv1a(pattern);
    if let Some(map) = bpf.map_mut("sensitive_paths") {
        if let Ok(mut hmap) = HashMap::<_, u32, u32>::try_from(map) {
            hmap.insert(hash, 1, 0)?;
        }
    }
    Ok(())
}

/// Block an IP address in the eBPF map
pub fn block_ip(bpf: &Bpf, ip: u32) -> Result<(), anyhow::Error> {
    if let Some(map) = bpf.map_mut("blocked_ips") {
        if let Ok(mut hmap) = HashMap::<_, u32, u32>::try_from(map) {
            hmap.insert(ip, 1, 0)?;
        }
    }
    Ok(())
}

/// Get current violation count from eBPF
pub fn get_violation_count(bpf: &Bpf) -> Result<u64, anyhow::Error> {
    if let Some(map) = bpf.map_mut("aegis_violations") {
        if let Ok(mut hmap) = PerCpuArray::<_, u64>::try_from(map) {
            let values = hmap.get(&0, 0)?;
            let total: u64 = values.iter().sum();
            return Ok(total);
        }
    }
    Ok(0)
}

fn calculate_fnv1a(input: &str) -> u32 {
    const FNV_OFFSET: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;
    let mut hash = FNV_OFFSET;
    for byte in input.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

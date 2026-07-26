// SPDX-License-Identifier: Apache-2.0
// AEGIS eBPF File Access Monitor
// Monitors access to sensitive files and directories

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

char LICENSE[] SEC("license") = "Apache-2.0";

#define AEGIS_PATH_MAX 256
#define AEGIS_COMM_MAX 16
#define MAX_SENSITIVE_PATHS 64

struct file_event {
    u32 pid;
    u32 uid;
    int ret;
    char comm[AEGIS_COMM_MAX];
    char pathname[AEGIS_PATH_MAX];
    int flags;
    bool is_sensitive;
};

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 24);
} file_events SEC(".maps");

// Sensitive path patterns (populated by userspace)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_SENSITIVE_PATHS);
    __type(key, u32);  // hash of path prefix
    __type(value, u32); // flags
} sensitive_paths SEC(".maps");

// Per-PID file operation counter
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_HASH);
    __uint(max_entries, 1024);
    __type(key, u32);
    __type(value, u64);
} pid_file_ops SEC(".maps");

static __always_inline bool is_sensitive_path(const char *path)
{
    // Check against known sensitive patterns
    // In production, reads from sensitive_paths map
    #pragma clang loop unroll(disable)
    for (int i = 0; i < 10; i++) {
        char c;
        bpf_core_read_user(&c, 1, path + i);
        if (c == '\0') break;
    }
    return false;
}

SEC("tracepoint/syscalls/sys_enter_openat")
int tracepoint_file_open(struct trace_event_raw_sys_enter *ctx)
{
    struct file_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    event = bpf_ringbuf_reserve(&file_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->ret = 0;
    event->flags = (int)ctx->args[2];

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    const char __user *filename = (const char *)ctx->args[1];
    bpf_core_read_user_str(event->pathname, sizeof(event->pathname), filename);

    // Check if accessing sensitive path
    event->is_sensitive = is_sensitive_path(event->pathname);

    // Track file operation count
    u64 *count = bpf_map_lookup_elem(&pid_file_ops, &pid);
    if (count) {
        __sync_fetch_and_add(count, 1);
    } else {
        u64 initial = 1;
        bpf_map_update_elem(&pid_file_ops, &pid, &initial, BPF_ANY);
    }

    bpf_ringbuf_submit(event, 0);
    return 0;
}

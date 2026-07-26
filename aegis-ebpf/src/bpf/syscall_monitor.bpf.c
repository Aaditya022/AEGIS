// SPDX-License-Identifier: Apache-2.0
// AEGIS eBPF Syscall Monitor
// Hooks critical syscalls for infrastructure-plane policy enforcement

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

char LICENSE[] SEC("license") = "Apache-2.0";

// ── Event Definitions ──────────────────────────────────────────────────

#define AEGIS_EVENT_MAX 256
#define AEGIS_PATH_MAX 256
#define AEGIS_COMM_MAX 16

enum aegis_event_type {
    AEGIS_EVENT_OPEN = 1,
    AEGIS_EVENT_CONNECT = 2,
    AEGIS_EVENT_EXECVE = 3,
    AEGIS_EVENT_UNLINK = 4,
    AEGIS_EVENT_WRITE = 5,
    AEGIS_EVENT_RENAME = 6,
    AEGIS_EVENT_SEND = 7,
    AEGIS_EVENT_RECV = 8,
    AEGIS_EVENT_CRED_ACCESS = 9,
};

struct aegis_event {
    enum aegis_event_type type;
    u32 pid;
    u32 uid;
    u32 gid;
    char comm[AEGIS_COMM_MAX];
    int ret;
    union {
        struct {
            char filename[AEGIS_PATH_MAX];
            int flags;
        } file;
        struct {
            u32 saddr;
            u32 daddr;
            u16 dport;
            u16 sport;
        } net;
        struct {
            char filename[AEGIS_PATH_MAX];
        } cred;
    } data;
};

// ── Maps ────────────────────────────────────────────────────────────────

// Ring buffer for events (Linux 5.8+)
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 24); // 16MB
} aegis_events SEC(".maps");

// Allowed syscall whitelist (populated by userspace)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 1024);
    __type(key, u32);  // PID
    __type(value, u32); // flags
} aegis_allowed_pids SEC(".maps");

// Policy violations counter
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, u32);
    __type(value, u64);
} aegis_violations SEC(".maps");

// ── Syscall Tracepoints ────────────────────────────────────────────────

SEC("tracepoint/syscalls/sys_enter_openat")
int tracepoint_openat(struct trace_event_raw_sys_enter *ctx)
{
    struct aegis_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    // Check if this PID is being monitored
    u32 *allowed = bpf_map_lookup_elem(&aegis_allowed_pids, &pid);
    if (!allowed) {
        return 0; // Not monitored — skip
    }

    event = bpf_ringbuf_reserve(&aegis_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = AEGIS_EVENT_OPEN;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->ret = 0;

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    // Read filename from syscall arguments
    const char __user *filename = (const char *)ctx->args[1];
    bpf_core_read_user_str(event->data.file.filename, sizeof(event->data.file.filename), filename);
    event->data.file.flags = (int)ctx->args[2];

    bpf_ringbuf_submit(event, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_connect")
int tracepoint_connect(struct trace_event_raw_sys_enter *ctx)
{
    struct aegis_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    u32 *allowed = bpf_map_lookup_elem(&aegis_allowed_pids, &pid);
    if (!allowed) {
        return 0;
    }

    struct sockaddr __user *addr = (struct sockaddr *)ctx->args[1];
    struct sockaddr_in *sin = (struct sockaddr_in *)addr;
    u16 family;

    bpf_core_read_user(&family, sizeof(family), &sin->sin_family);
    if (family != AF_INET) {
        return 0;
    }

    event = bpf_ringbuf_reserve(&aegis_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = AEGIS_EVENT_CONNECT;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->ret = 0;

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    bpf_core_read_user(&event->data.net.daddr, sizeof(u32), &sin->sin_addr.s_addr);
    bpf_core_read_user(&event->data.net.dport, sizeof(u16), &sin->sin_port);

    bpf_ringbuf_submit(event, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_execve")
int tracepoint_execve(struct trace_event_raw_sys_enter *ctx)
{
    struct aegis_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    u32 *allowed = bpf_map_lookup_elem(&aegis_allowed_pids, &pid);
    if (!allowed) {
        return 0;
    }

    event = bpf_ringbuf_reserve(&aegis_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = AEGIS_EVENT_EXECVE;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->ret = 0;

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    const char __user *filename = (const char *)ctx->args[0];
    bpf_core_read_user_str(event->data.file.filename, sizeof(event->data.file.filename), filename);

    bpf_ringbuf_submit(event, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_unlinkat")
int tracepoint_unlinkat(struct trace_event_raw_sys_enter *ctx)
{
    struct aegis_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    u32 *allowed = bpf_map_lookup_elem(&aegis_allowed_pids, &pid);
    if (!allowed) {
        return 0;
    }

    event = bpf_ringbuf_reserve(&aegis_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = AEGIS_EVENT_UNLINK;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->ret = 0;

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    const char __user *filename = (const char *)ctx->args[1];
    bpf_core_read_user_str(event->data.file.filename, sizeof(event->data.file.filename), filename);

    bpf_ringbuf_submit(event, 0);
    return 0;
}

// SPDX-License-Identifier: Apache-2.0
// AEGIS eBPF TCP Monitor
// Monitors all TCP send/recv operations for data exfiltration detection

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

char LICENSE[] SEC("license") = "Apache-2.0";

#define AEGIS_EVENT_MAX 256
#define AEGIS_ADDR_STR_LEN 64
#define AEGIS_COMM_MAX 16

enum tcp_event_type {
    TCP_EVENT_SEND = 1,
    TCP_EVENT_RECV = 2,
};

struct tcp_event {
    enum tcp_event_type type;
    u32 pid;
    u32 uid;
    u32 saddr;
    u32 daddr;
    u16 sport;
    u16 dport;
    u64 bytes;
    char comm[AEGIS_COMM_MAX];
};

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 24);
} tcp_events SEC(".maps");

// TCP connection tracking
struct conn_key {
    u32 saddr;
    u32 daddr;
    u16 sport;
    u16 dport;
};

struct conn_info {
    u64 tx_bytes;
    u64 rx_bytes;
    u64 last_seen;
    u32 pid;
    u32 uid;
};

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, struct conn_key);
    __type(value, struct conn_info);
} conn_stats SEC(".maps");

// Blocked IP addresses (populated by userspace)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 1024);
    __type(key, u32);
    __type(value, u32);
} blocked_ips SEC(".maps");

SEC("tracepoint/syscalls/sys_enter_sendto")
int tracepoint_sendto(struct trace_event_raw_sys_enter *ctx)
{
    struct tcp_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    event = bpf_ringbuf_reserve(&tcp_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = TCP_EVENT_SEND;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->bytes = ctx->args[2]; // len

    struct sockaddr __user *addr = (struct sockaddr *)ctx->args[4];
    if (addr) {
        struct sockaddr_in *sin = (struct sockaddr_in *)addr;
        u16 family;
        bpf_core_read_user(&family, sizeof(family), &sin->sin_family);
        if (family == AF_INET) {
            bpf_core_read_user(&event->daddr, sizeof(u32), &sin->sin_addr.s_addr);
            bpf_core_read_user(&event->dport, sizeof(u16), &sin->sin_port);

            // Check if destination is blocked
            u32 *blocked = bpf_map_lookup_elem(&blocked_ips, &event->daddr);
            if (blocked) {
                // Report blocked attempt
                bpf_ringbuf_submit(event, 0);

                // Increment violation counter
                u32 key = 0;
                u64 *violations = bpf_map_lookup_elem(&conn_stats, &key);
                if (violations) {
                    __sync_fetch_and_add(violations, 1);
                }
                return 0;
            }
        }
    }

    bpf_get_current_comm(&event->comm, sizeof(event->comm));
    bpf_ringbuf_submit(event, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_recvfrom")
int tracepoint_recvfrom(struct trace_event_raw_sys_enter *ctx)
{
    struct tcp_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    event = bpf_ringbuf_reserve(&tcp_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->type = TCP_EVENT_RECV;
    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->bytes = ctx->args[2]; // len

    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    struct sockaddr __user *addr = (struct sockaddr *)ctx->args[5];
    if (addr) {
        struct sockaddr_in *sin = (struct sockaddr_in *)addr;
        u16 family;
        bpf_core_read_user(&family, sizeof(family), &sin->sin_family);
        if (family == AF_INET) {
            bpf_core_read_user(&event->daddr, sizeof(u32), &sin->sin_addr.s_addr);
            bpf_core_read_user(&event->dport, sizeof(u16), &sin->sin_port);
        }
    }

    bpf_ringbuf_submit(event, 0);
    return 0;
}

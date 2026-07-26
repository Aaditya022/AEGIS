// SPDX-License-Identifier: Apache-2.0
// AEGIS eBPF Credential Monitor
// Detects unauthorized credential/secret access

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

char LICENSE[] SEC("license") = "Apache-2.0";

#define AEGIS_PATH_MAX 256
#define AEGIS_COMM_MAX 16

struct cred_event {
    u32 pid;
    u32 uid;
    u32 gid;
    char comm[AEGIS_COMM_MAX];
    char filename[AEGIS_PATH_MAX];
    int access_type;  // 0: read, 1: write, 2: stat
};

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 24);
} cred_events SEC(".maps");

// Known credential file patterns
static const char *cred_patterns[] = {
    ".env",
    "credentials",
    "secret",
    "token",
    "key.json",
    "id_rsa",
    "password",
    ".kube/config",
    "aws/credentials",
    "config.json",
    "service-account",
    "api-key",
};

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 64);
    __type(key, u32);
    __type(value, u32);
} cred_blocklist SEC(".maps");

static __always_inline bool is_credential_file(const char *path)
{
    if (!path) return false;

    // Check against credential patterns
    #pragma clang loop unroll(disable)
    for (int i = 0; i < 11; i++) {
        const char *pattern = cred_patterns[i];
        bool match = true;

        // Simple substring match
        #pragma clang loop unroll(disable)
        for (int j = 0; j < 64; j++) {
            char pc, pp;
            bpf_core_read_user(&pc, 1, path + j);
            bpf_core_read(&pp, 1, pattern + j);

            if (pp == '\0') {
                if (match) return true;
                break;
            }
            if (pc == '\0') {
                match = false;
                break;
            }

            char lower = (pc >= 'A' && pc <= 'Z') ? pc + 32 : pc;
            char plower = (pp >= 'A' && pp <= 'Z') ? pp + 32 : pp;
            if (lower != plower) {
                match = false;
            }
        }
    }
    return false;
}

SEC("tracepoint/syscalls/sys_enter_openat")
int tracepoint_cred_access(struct trace_event_raw_sys_enter *ctx)
{
    struct cred_event *event;
    pid_t pid = bpf_get_current_pid_tgid() >> 32;

    const char __user *filename = (const char *)ctx->args[1];
    char buf[AEGIS_PATH_MAX];

    bpf_core_read_user_str(buf, sizeof(buf), filename);

    if (!is_credential_file(buf)) {
        return 0;
    }

    event = bpf_ringbuf_reserve(&cred_events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }

    event->pid = pid;
    event->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
    event->gid = (bpf_get_current_uid_gid() >> 32) & 0xFFFFFFFF;
    event->access_type = 0;  // read

    bpf_get_current_comm(&event->comm, sizeof(event->comm));
    __builtin_memcpy(event->filename, buf, sizeof(buf));

    bpf_ringbuf_submit(event, 0);
    return 0;
}

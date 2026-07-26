use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bpf_dir = dir.join("src/bpf");

    let bpf_programs = [
        "syscall_monitor.bpf.c",
        "tcp_monitor.bpf.c",
        "file_access.bpf.c",
        "credential_monitor.bpf.c",
    ];

    let clang_available = Command::new("clang").arg("--version").output().is_ok();

    fs::create_dir_all(&bpf_dir).ok();

    for program in &bpf_programs {
        let src = bpf_dir.join(program);
        let out = bpf_dir.join(program.replace(".bpf.c", ".o"));

        println!("cargo:rerun-if-changed={}", src.display());

        if clang_available && src.exists() {
            let status = Command::new("clang")
                .args([
                    "-O2",
                    "-target",
                    "bpf",
                    "-c",
                    &src.to_string_lossy(),
                    "-o",
                    &out.to_string_lossy(),
                ])
                .status()
                .expect("clang execution failed");

            if status.success() {
                println!("cargo:warning=Compiled {}", program);
                continue;
            }
        }

        // Create stub ELF file so include_bytes_aligned! doesn't fail
        if !out.exists() {
            // Minimal valid ELF-64 object file header
            let elf_header: Vec<u8> = vec![
                0x7f, 0x45, 0x4c, 0x46, // ELF magic
                0x02, // 64-bit
                0x01, // little-endian
                0x01, // ELF version
                0x00, // OS/ABI (ELFOSABI_NONE)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
                0x01, 0x00, // ET_REL (relocatable)
                0xf7, 0x00, // EM_BPF
                0x01, 0x00, 0x00, 0x00, // EV_CURRENT
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry (0)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // phoff (0)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // shoff (0)
                0x00, 0x00, 0x00, 0x00, // flags
                0x40, 0x00, // ehsize (64)
                0x00, 0x00, // phentsize (0)
                0x00, 0x00, // phnum (0)
                0x00, 0x00, // shentsize (0)
                0x00, 0x00, // shnum (0)
                0x00, 0x00, // shstrndx (0)
            ];
            fs::write(&out, elf_header).ok();
            println!(
                "cargo:warning=Created stub {}.o (clang not available)",
                program.replace(".bpf.c", "")
            );
        }
    }

    #[cfg(target_os = "macos")]
    println!("cargo:warning=eBPF requires Linux kernel 5.19+. Running in simulated mode.");
}

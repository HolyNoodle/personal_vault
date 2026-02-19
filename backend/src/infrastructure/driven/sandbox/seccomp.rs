/// BPF-based seccomp syscall denylist, implemented with raw libc types.
///
/// Strategy: allow all syscalls by default, deny dangerous ones with EPERM.
/// The BPF program is built in the parent process and passed to the child's
/// pre_exec closure as a plain Vec.

use libc::{sock_filter, sock_fprog, PR_SET_SECCOMP};

// BPF instruction class constants
const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JEQ: u16 = 0x10;
const BPF_RET: u16 = 0x06;
const BPF_K: u16 = 0x00;

// seccomp return codes
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_ERRNO_BASE: u32 = 0x0005_0000;

// arch check: AUDIT_ARCH_X86_64 = 0xC000003E
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;

// offsets in struct seccomp_data
const SECCOMP_DATA_NR_OFFSET: u32 = 0; // syscall nr
const SECCOMP_DATA_ARCH_OFFSET: u32 = 4; // architecture

fn bpf_stmt(code: u16, k: u32) -> sock_filter {
    sock_filter { code, jt: 0, jf: 0, k }
}

fn bpf_jump(code: u16, k: u32, jt: u8, jf: u8) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// Build a seccomp BPF program as a Vec<sock_filter>.
/// Returned value is moved into the pre_exec closure (it's plain C structs — Send + Sync).
pub fn build_seccomp_filter() -> Vec<sock_filter> {
    // Denied syscalls → return EPERM
    let denied: &[i64] = &[
        libc::SYS_ptrace,
        libc::SYS_kexec_load,
        libc::SYS_init_module,
        libc::SYS_finit_module,
        libc::SYS_delete_module,
        libc::SYS_setuid,
        libc::SYS_setgid,
        libc::SYS_setreuid,
        libc::SYS_setregid,
        libc::SYS_setresuid,
        libc::SYS_setresgid,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_perf_event_open,
    ];

    let n = denied.len() as u8;
    let mut prog: Vec<sock_filter> = Vec::with_capacity(4 + denied.len() + 1);

    // Instruction 0: load architecture field
    prog.push(bpf_stmt(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_ARCH_OFFSET));
    // Instruction 1: if arch != x86_64, jump to ALLOW.
    // jf is relative to the *next* instruction (index 2).
    // ALLOW is at index 3+n, so jf = (3+n) - 2 = n+1.
    let allow_offset = n + 1;
    prog.push(bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 0, allow_offset));

    // Instruction 2: load syscall number
    prog.push(bpf_stmt(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));

    // Instructions 3..3+n: check each denied syscall
    for (i, &syscall) in denied.iter().enumerate() {
        let remaining = (n as u8) - (i as u8) - 1;
        // if syscall_nr == denied[i]: return EPERM
        // else: continue to next check (or ALLOW)
        prog.push(bpf_jump(
            BPF_JMP | BPF_JEQ | BPF_K,
            syscall as u32,
            remaining + 1, // jt: jump to EPERM return at end
            0,             // jf: fall through to next instruction
        ));
    }

    // ALLOW instruction
    prog.push(bpf_stmt(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    // EPERM instruction
    prog.push(bpf_stmt(BPF_RET | BPF_K, SECCOMP_RET_ERRNO_BASE | (libc::EPERM as u32)));

    prog
}

/// Install the seccomp filter in the current thread/process.
/// Must be called from within the pre_exec closure.
///
/// # Safety
/// Caller must ensure this is called from a single-threaded child process
/// (i.e., from within a pre_exec closure).
pub unsafe fn apply_seccomp_filter(prog: &[sock_filter]) -> std::io::Result<()> {
    let fprog = sock_fprog {
        len: prog.len() as u16,
        filter: prog.as_ptr() as *mut sock_filter,
    };

    // PR_SET_NO_NEW_PRIVS: required before SECCOMP_MODE_FILTER unless already root
    if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
        return Err(std::io::Error::last_os_error());
    }

    // SECCOMP_MODE_FILTER = 2
    const SECCOMP_MODE_FILTER: libc::c_ulong = 2;
    let fprog_ptr = &fprog as *const sock_fprog as libc::c_ulong;
    if libc::prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, fprog_ptr, 0, 0) != 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

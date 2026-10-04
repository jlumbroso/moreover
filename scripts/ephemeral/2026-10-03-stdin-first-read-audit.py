#!/usr/bin/env python3
"""Probe the repaired stdin helper's first-read and drain error boundaries.

Authorship: GPT-6 Astra delegated reviewer (post_claims), for Lector 6.
Run: python3 scripts/ephemeral/2026-10-03-stdin-first-read-audit.py

Copies the pinned source unchanged into owned disposable scratch, appends
helper-level tests, and runs only those tests. No product source is edited.
Requires git, cargo, and locally available offline build dependencies.

The initial Interrupted case is a synthetic io::Read result, not a native
signal or character-device reproduction. It compares the former standard
read_to_end behavior with the repaired helper. Other cases check exact
head/rest joining and ordinary errors before and after initial data.
Historical audit assertions describe the pinned implementation; they are
not passing regressions for a future repair.

Successfully run 2026-10-03: all three probe tests passed. The helper
joined 200,003 bytes exactly, propagated both hard-error cases as exit 1,
and treated the injected first-read Interrupted as exit 1 while ordinary
read_to_end retried that same scripted reader and completed its input.
"""

from pathlib import Path
import subprocess
import tempfile


REVISION = "3cbce1c5e7749141ee405639f068f1764c7bb115"

TEST = r'''
#[cfg(test)]
mod audit_first_read {
    use super::*;
    use std::time::Duration;

    struct InterruptedFirst {
        first: bool,
        data: io::Cursor<Vec<u8>>,
    }
    impl InterruptedFirst {
        fn new() -> Self {
            Self { first: true, data: io::Cursor::new(b"valid input\n".to_vec()) }
        }
    }
    impl io::Read for InterruptedFirst {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.first {
                self.first = false;
                return Err(io::Error::new(io::ErrorKind::Interrupted, "synthetic interruption"));
            }
            self.data.read(buf)
        }
    }

    #[test]
    fn initial_interrupted_read_is_fatal_only_in_the_new_helper() {
        let mut ordinary = Vec::new();
        InterruptedFirst::new().read_to_end(&mut ordinary).unwrap();
        assert_eq!(ordinary, b"valid input\n");

        let (code, message) = read_bounded(InterruptedFirst::new(), Duration::from_secs(1))
            .expect_err("pinned initial read does not retry Interrupted");
        assert_eq!(code, 1);
        assert!(message.contains("synthetic interruption"));
        println!("standard_read_to_end=complete helper_initial_interrupted=exit{code} {message}");
    }

    #[test]
    fn head_and_large_remainder_are_joined_exactly() {
        let bytes: Vec<u8> = (0..200_003).map(|i| (i % 251) as u8).collect();
        let output = read_bounded(io::Cursor::new(bytes.clone()), Duration::from_secs(1)).unwrap();
        assert_eq!(output, bytes);
        println!("head_and_rest_exact_bytes={}", output.len());
    }

    struct HardError { initial_data: bool }
    impl io::Read for HardError {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.initial_data {
                self.initial_data = false;
                buf[..3].copy_from_slice(b"abc");
                return Ok(3);
            }
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "synthetic hard error"))
        }
    }

    #[test]
    fn hard_errors_before_and_after_initial_data_are_not_silence() {
        for initial_data in [false, true] {
            let (code, message) = read_bounded(HardError { initial_data }, Duration::from_secs(1))
                .expect_err("hard errors must propagate");
            assert_eq!(code, 1);
            assert!(message.contains("synthetic hard error"));
            println!("hard_error_after_initial_data={initial_data} exit={code}");
        }
    }
}
'''


def main():
    probe_dir = Path(__file__).resolve().parent
    repository = probe_dir.parents[1]
    with tempfile.TemporaryDirectory(prefix=".2026-10-03-stdin-first-read-", dir=probe_dir) as temporary:
        source = Path(temporary) / "source"
        names = subprocess.check_output(
            ["git", "ls-tree", "-r", "--name-only", REVISION,
             "--", "Cargo.toml", "Cargo.lock", "src"],
            cwd=repository, text=True,
        ).splitlines()
        for name in names:
            target = source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(subprocess.check_output(
                ["git", "show", f"{REVISION}:{name}"], cwd=repository,
            ))
        with (source / "src/main.rs").open("a") as output:
            output.write(TEST)
        print(f"reviewed_commit={REVISION}; production_functions_unchanged=true", flush=True)
        subprocess.run(
            ["cargo", "test", "--offline", "--locked", "--quiet",
             "--manifest-path", str(source / "Cargo.toml"),
             "--target-dir", str(Path(temporary) / "target"),
             "--bin", "moreover", "audit_first_read", "--", "--nocapture"],
            check=True, timeout=120,
        )


if __name__ == "__main__":
    main()

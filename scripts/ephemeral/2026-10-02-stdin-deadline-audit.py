#!/usr/bin/env python3
"""Check the new stdin deadline after bytes arrive but before EOF.

Authorship: Lector 6 (GPT-6 Astra; gpt-6-astra).
Run: python3 scripts/ephemeral/2026-10-02-stdin-deadline-audit.py

Copies pinned source unchanged, then appends one test module to main.rs
in an owned disposable checkout below scripts/ephemeral. The test calls
the actual read_bounded helper with the production read_to_end operation.
A Unix socket supplies real bytes immediately and withholds EOF; a small
Read wrapper counts consumed bytes without changing them. The helper
gets a shortened 200ms deadline, not a changed implementation.

This isolates the bounded-reader mechanism. It does not claim to test a
live harness character device or stdin_is_char_device's classification.
All source, binaries, and fixtures are removed on exit. Requires git,
cargo, Unix sockets, and offline build dependencies already available.

Successfully run 2026-10-02: the reader consumed all 33 supplied bytes
before timing out with exit 2 and the "neither data nor end-of-file"
diagnostic. EOF afterward completed the finite drain. This supports
timing first activity rather than the whole drain. Historical audit
evidence, not a regression intended to pass on subsequently fixed code.
"""

from pathlib import Path
import subprocess
import tempfile


REVISION = "3309c5c84a22b64c85b54e45e56b340b4c16faab"

TEST = r'''
#[cfg(all(test, unix))]
mod audit_delayed_eof {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}, mpsc};
    use std::time::Duration;

    struct ObservedRead {
        stream: UnixStream,
        bytes: Arc<AtomicUsize>,
    }
    impl Read for ObservedRead {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.stream.read(buf)?;
            self.bytes.fetch_add(n, Ordering::SeqCst);
            Ok(n)
        }
    }

    #[test]
    fn bytes_before_deadline_still_receive_the_silence_error() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        let early = b"data arrived before the deadline\n";
        writer.write_all(early).unwrap();
        let seen = Arc::new(AtomicUsize::new(0));
        let mut observed = ObservedRead { stream: reader, bytes: seen.clone() };
        let (finished_tx, finished_rx) = mpsc::channel();
        let result = read_bounded(move || {
            let mut buf = Vec::new();
            // The same complete-drain operation used by read_stdin_guarded.
            observed.read_to_end(&mut buf)?;
            finished_tx.send(buf.clone()).unwrap();
            Ok(buf)
        }, Duration::from_millis(200));

        let consumed = seen.load(Ordering::SeqCst);
        let (code, message) = result.expect_err("pinned helper times the complete drain");
        assert_eq!(consumed, early.len(), "bytes must actually have arrived before timeout");
        assert_eq!(code, 2);
        assert!(message.contains("neither data nor end-of-file"));
        println!("consumed_before_timeout={consumed} exit={code} diagnostic={message:?}");

        // Deliver EOF after the deadline and wait for the reader to finish;
        // this is a finite delayed input, not an unbounded producer.
        drop(writer);
        let drained = finished_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(drained, early);
        println!("eof_after_deadline=true drained_bytes={} caller_result_was_error=true", drained.len());
    }
}
'''


def main():
    home = Path(__file__).resolve().parent
    repo = home.parents[1]
    with tempfile.TemporaryDirectory(prefix=".stdin-deadline-audit-", dir=home) as temporary:
        source = Path(temporary) / "source"
        paths = subprocess.check_output(
            ["git", "ls-tree", "-r", "--name-only", REVISION,
             "--", "Cargo.toml", "Cargo.lock", "src"], cwd=repo, text=True,
        ).splitlines()
        for name in paths:
            destination = source / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(subprocess.check_output(
                ["git", "show", f"{REVISION}:{name}"], cwd=repo,
            ))
        with (source / "src/main.rs").open("a") as stream:
            stream.write(TEST)
        print(f"reviewed_commit={REVISION}; production_functions_unchanged=true", flush=True)
        subprocess.run(
            ["cargo", "test", "--offline", "--locked", "--quiet",
             "--manifest-path", str(source / "Cargo.toml"),
             "--target-dir", str(Path(temporary) / "target"),
             "--bin", "moreover", "audit_delayed_eof", "--", "--nocapture"],
            check=True, timeout=120,
        )


if __name__ == "__main__":
    main()

// Binary-level acceptance: the flags a reader actually types, run against
// the real executable (CARGO_BIN_EXE_) with isolated state dirs. These
// exist because the library tests cannot see stream routing — and the
// trailer's destination is the whole point of --trailer (ADR-0003).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static SCRATCH_SEQ: AtomicU64 = AtomicU64::new(0);

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "moreover-cli-{}-{}-{}",
        tag,
        std::process::id(),
        SCRATCH_SEQ.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(state: &PathBuf, args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
    cmd.args(args)
        .env("MOREOVER_STATE_DIR", state)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    if let Some(bytes) = stdin {
        child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
    }
    child.wait_with_output().unwrap()
}

fn lines(n: usize) -> Vec<u8> {
    (1..=n).map(|i| format!("l{i}\n")).collect::<String>().into_bytes()
}

#[test]
fn trailer_default_is_stderr_and_stdout_stays_pure() {
    let state = scratch_dir("stderr");
    let out = run(&state, &["-3"], Some(&lines(9)));
    assert_eq!(out.stdout, lines(3), "stdout must be pure content");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.starts_with("<moreover: page 1, 3/9 lines, cursor: "), "got: {err}");
}

#[test]
fn trailer_stdout_moves_it_for_stdout_only_harnesses() {
    let state = scratch_dir("stdout");
    let out = run(&state, &["-3", "--trailer", "stdout"], Some(&lines(9)));
    assert!(out.stderr.is_empty(), "stderr must be silent under --trailer stdout");
    let sout = String::from_utf8(out.stdout).unwrap();
    assert!(sout.starts_with("l1\nl2\nl3\n<moreover: page 1, 3/9 lines"), "got: {sout}");
}

#[test]
fn trailer_none_hides_it_entirely() {
    let state = scratch_dir("none");
    let out = run(&state, &["-3", "--trailer", "none"], Some(&lines(9)));
    assert_eq!(out.stdout, lines(3));
    assert!(out.stderr.is_empty());
}

#[test]
fn trailer_file_appends_to_a_path() {
    let state = scratch_dir("tfile");
    let sink = state.join("trailers.log");
    let sink_arg = format!("file:{}", sink.display());
    run(&state, &["-3", "--trailer", &sink_arg], Some(&lines(9)));
    run(&state, &["-3", "--trailer", &sink_arg], Some(&lines(9)));
    let logged = std::fs::read_to_string(&sink).unwrap();
    assert_eq!(logged.matches("<moreover:").count(), 2, "file dest must append");
}

#[test]
fn file_mode_pages_a_file_like_more_always_did() {
    let state = scratch_dir("fmode");
    let doc = state.join("doc.txt");
    std::fs::write(&doc, lines(7)).unwrap();
    let out = run(&state, &[doc.to_str().unwrap(), "-4"], None);
    assert_eq!(out.stdout, lines(4));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("4/7 lines"), "got: {err}");
    assert!(out.status.success());
}

#[test]
fn resume_rejects_an_input_file() {
    // A cursor already names its stream; accepting a file too would
    // silently ignore one of the two — reject instead (strict syntactic
    // boundary, ADR-0003 design commitment 2).
    let state = scratch_dir("conflict");
    let out = run(&state, &["-c", "ae2e", "somefile.txt"], None);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn contract_subcommand_carries_the_promises() {
    // The two-world grammar (ADR-0003 QST-SUBCOMMAND-GRAMMAR, accepted):
    // the contract is a desk subcommand, not a flag.
    let state = scratch_dir("contract");
    let out = run(&state, &["contract"], None);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    // The lines a model reader most needs, verbatim commitments:
    assert!(text.contains("cursor: {cursor}"), "trailer template missing");
    assert!(text.contains("never invent or extrapolate one"), "don't-invent rule missing");
    assert!(text.contains("resuming the same cursor twice yields the same page"));
}

#[test]
fn desk_verbs_are_reserved_and_teach_the_escape() {
    // A file named `ls` must never silently page; the error must teach
    // `./ls` (design commitment 3: errors are written to the reader).
    let state = scratch_dir("reserved");
    for verb in ["ls", "stat", "drop", "gc"] {
        let out = run(&state, &[verb], None);
        assert_eq!(out.status.code(), Some(2), "{verb} must be reserved");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains(&format!("./{verb}")), "error must teach the escape: {err}");
    }
}

#[test]
fn unknown_cursor_error_is_written_to_the_reader() {
    let state = scratch_dir("nocursor");
    let out = run(&state, &["-c", "zz9q", "--all"], None);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("only valid if moreover printed it"),
        "the error must teach the rule, not just refuse: {err}"
    );
}

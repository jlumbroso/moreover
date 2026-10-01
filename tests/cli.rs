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
        .env_remove("MOREOVER_TRAILER") // a dev's standing default must not skew tests
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
#[cfg(unix)]
fn shell_redirection_can_merge_or_discard_the_continuation() {
    // A caller discarded stderr out of habit and lost the continuation.
    // Exercise actual shell redirections: merging preserves the trailer
    // but mixes it with content; routing the trailer to stdout survives
    // stderr suppression without restoring a content-only stdout.
    let state = scratch_dir("redirections");
    let doc = state.join("input.txt");
    std::fs::write(&doc, lines(5)).unwrap();
    let invoke = |script: &str| {
        Command::new("/bin/sh")
            .args(["-c", script, "moreover-redirection-test"])
            .arg(env!("CARGO_BIN_EXE_moreover"))
            .arg(&doc)
            .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };

    let separate = invoke("\"$1\" \"$2\" -2");
    let merged = invoke("\"$1\" \"$2\" -2 2>&1");
    let discarded = invoke("\"$1\" \"$2\" -2 2>/dev/null");
    let stdout_trailer = invoke("\"$1\" \"$2\" -2 --trailer stdout 2>/dev/null");

    for output in [&separate, &merged, &discarded, &stdout_trailer] {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    assert_eq!(separate.stdout, lines(2));
    assert!(String::from_utf8_lossy(&separate.stderr)
        .starts_with("<moreover: page 1, 2/5 lines, cursor: "));
    assert_eq!(discarded.stdout, lines(2));
    assert!(discarded.stderr.is_empty());
    for output in [&merged, &stdout_trailer] {
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout)
            .starts_with("l1\nl2\n<moreover: page 1, 2/5 lines, cursor: "));
    }
}

#[test]
fn env_sets_the_trailer_default_and_the_flag_beats_it() {
    // ADR-0003 QST-ENV-OVERRIDE, his acceptance ("the precedence in
    // Option A is just right"): MOREOVER_TRAILER moves the default; an
    // explicit --trailer always wins; an invalid env value errors loudly
    // rather than silently steering the most-seen surface.
    let state = scratch_dir("envdefault");
    let with_env = |extra: &[&str], env_val: &str, stdin: &[u8]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(extra)
            .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
            .env("MOREOVER_TRAILER", env_val)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().unwrap();
        child.stdin.as_mut().unwrap().write_all(stdin).unwrap();
        child.wait_with_output().unwrap()
    };

    let env_only = with_env(&["-3"], "stdout", &lines(9));
    assert!(env_only.stderr.is_empty(), "env default must move the trailer off stderr");
    assert!(String::from_utf8_lossy(&env_only.stdout).contains("<moreover:"));

    let flag_wins = with_env(&["-3", "--trailer", "stderr"], "stdout", &lines(9));
    assert!(String::from_utf8_lossy(&flag_wins.stderr).starts_with("<moreover:"));
    assert_eq!(flag_wins.stdout, lines(3), "flag must beat env");

    let invalid = with_env(&["-3"], "sdtout", &lines(9));
    assert_eq!(invalid.status.code(), Some(2), "typo'd env must error, not fall back");
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("MOREOVER_TRAILER"));

    // Regression (Lector 6's audit of 1df18dd): the env was parsed
    // eagerly, so a typo'd MOREOVER_TRAILER broke even invocations that
    // carried an explicit --trailer — and blocked --help and --version.
    // The flag must win over an INVALID env too (the variable is only
    // read when no flag is given), and the introspection surfaces must
    // never touch it.
    let flag_over_bad_env = with_env(&["-3", "--trailer", "stderr"], "sdtout", &lines(9));
    assert!(flag_over_bad_env.status.success(), "explicit flag must beat a typo'd env");
    assert!(String::from_utf8_lossy(&flag_over_bad_env.stderr).starts_with("<moreover:"));
    assert_eq!(flag_over_bad_env.stdout, lines(3));

    for introspection in [&["--help"][..], &["--version"][..], &["contract"][..]] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(introspection)
            .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
            .env("MOREOVER_TRAILER", "sdtout")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "{introspection:?} must work under a broken env: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
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
    assert!(text.contains("same page size, unit, and overlap"), "replay conditions missing");
    assert!(text.contains("repeats the same content"), "content replay commitment missing");
    assert!(text.contains("The next cursor ID may differ"), "cursor identity caveat missing");
}

#[test]
fn resume_uses_this_invocations_size_unit_and_overlap() {
    // Regression: the contract promised the same page for any replay,
    // but a cursor stores a byte position, not the caller's paging options.
    // Resume from inside a line to make that distinction observable.
    let state = scratch_dir("replay-options");
    let first = run(&state, &["--bytes", "2"], Some(b"alpha\nbeta\ngamma\ndelta\n"));
    assert!(first.status.success());
    assert_eq!(first.stdout, b"al");
    let trailer = String::from_utf8(first.stderr).unwrap();
    let cursor = trailer.split("cursor: ").nth(1).unwrap().trim_end_matches(">\n");

    let line = run(&state, &["-c", cursor, "-1"], None);
    let replay = run(&state, &["-c", cursor, "-1"], None);
    let bytes = run(&state, &["-c", cursor, "--bytes", "3"], None);
    let overlap = run(&state, &["-c", cursor, "--bytes", "3", "--overlap", "2"], None);
    let defaults = run(&state, &["-c", cursor], None);

    for output in [&line, &replay, &bytes, &overlap, &defaults] {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    assert_eq!(line.stdout, b"pha\n");
    assert_eq!(replay.stdout, line.stdout, "same options must repeat content");
    assert_eq!(bytes.stdout, b"pha");
    assert_eq!(overlap.stdout, b"alpha", "byte overlap must prepend saved bytes");
    assert_eq!(defaults.stdout, b"pha\nbeta\ngamma\ndelta\n", "resume defaults to lines");
}

#[test]
fn desk_verbs_are_reserved_and_teach_the_escape() {
    // A file named `stat` must never silently page; the error must teach
    // `./stat` (design commitment 3: errors are written to the reader).
    // ls/drop/gc graduated from reserved to implemented (ADR-0005), so
    // only stat still carries the reservation error — but a bad desk
    // invocation must STILL never fall through to paging.
    let state = scratch_dir("reserved");
    let out = run(&state, &["stat"], None);
    assert_eq!(out.status.code(), Some(2), "stat must be reserved");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("./stat"), "error must teach the escape: {err}");

    // drop without an id is a desk usage error, never a page of a file
    // named 'drop'
    let out = run(&state, &["drop"], None);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("one cursor ID"));
}

#[test]
#[cfg(target_os = "macos")]
fn bare_call_on_a_terminal_guides_instead_of_hanging() {
    // Regression (first dogfooding seed, 2026-09-23): the very first
    // thing the first human dogfooder typed was `moreover`, alone, on a
    // terminal — and it hung reading stdin until ^C, like cat/head/tail
    // and unlike less/more, which check for a terminal. Fixed by the
    // null-call guide (two-audience pointer, exit 2). `script -q` gives
    // the binary a real pty, so this runs the exact keystroke he ran;
    // macOS-gated because linux `script` takes different arguments.
    let state = scratch_dir("nullcall");
    let out = Command::new("script")
        .args(["-q", "/dev/null", env!("CARGO_BIN_EXE_moreover")])
        .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    // The pty merges streams into the transcript; assert on content.
    let transcript = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(transcript.contains("for humans"), "guide missing human pointer: {transcript}");
    assert!(transcript.contains("for models"), "guide missing model pointer: {transcript}");
    assert!(
        !out.status.success(),
        "a null call is a usage miss, not a success"
    );
}

#[test]
fn piped_empty_input_is_not_a_null_call() {
    // The inductive base case survives the null-call guide: an EMPTY
    // stream is still a stream — `true | moreover` earns its 0/0
    // trailer, because the guide fires on a terminal stdin, never on a
    // pipe that happened to carry nothing.
    let state = scratch_dir("emptypipe");
    let out = run(&state, &["-10"], Some(b""));
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("0/0 lines, cursor: null"), "got: {err}");
}

#[test]
fn c_last_resumes_this_desks_newest_cursor_only() {
    // Regression for a second model reader's first-contact failure
    // (field report, 2026-09-23): their first invocation appended
    // 2>/dev/null out of trained habit, destroying the trailer and
    // orphaning a cursor they never saw — and v0.2.0 had no in-band
    // recovery. `-c last` is that recovery, desk-scoped so "my last"
    // never resumes a concurrent reader's stream from another directory.
    let state = scratch_dir("last");
    let desk_a = scratch_dir("last-desk-a");
    let desk_b = scratch_dir("last-desk-b");
    let doc = desk_a.join("doc.txt");
    std::fs::write(&doc, lines(9)).unwrap();

    // Mint from desk A (trailer discarded — the F1 habit, simulated).
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
    cmd.args([doc.to_str().unwrap(), "-3"])
        .current_dir(&desk_a)
        .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    assert!(cmd.status().unwrap().success());

    // Recovery from desk A: last finds the orphaned cursor.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
    cmd.args(["-c", "last", "-3"])
        .current_dir(&desk_a)
        .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, lines(9)[9..18].to_vec(), "lines 4..6 expected");

    // Desk B minted nothing: last must refuse and teach, not guess.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
    cmd.args(["-c", "last", "-3"])
        .current_dir(&desk_b)
        .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("no cursors were minted from this directory"), "got: {err}");
}

#[test]
fn c_last_remains_selectable_after_exhaustion_and_complete_new_input() {
    // `last` looks up a saved cursor, not the last invocation's completion
    // state. Finishing a stream, or fully consuming a new short input,
    // mints no successor: another `last` can replay the earlier remainder.
    let state = scratch_dir("last-completed");
    let desk = scratch_dir("last-completed-desk");
    let doc = desk.join("input.txt");
    let short = desk.join("short.txt");
    std::fs::write(&doc, lines(5)).unwrap();
    std::fs::write(&short, b"new input\n").unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_moreover"))
            .args(args)
            .current_dir(&desk)
            .env("MOREOVER_STATE_DIR", &state).env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };

    let first = invoke(&[doc.to_str().unwrap(), "-2"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, lines(2));
    let remainder = b"l3\nl4\nl5\n";
    for _ in 0..2 {
        let finished = invoke(&["-c", "last", "--all"]);
        assert!(finished.status.success());
        assert_eq!(finished.stdout, remainder);
        assert!(String::from_utf8_lossy(&finished.stderr).contains("cursor: null>"));
    }

    let complete_new_input = invoke(&[short.to_str().unwrap(), "--all"]);
    assert!(complete_new_input.status.success());
    assert_eq!(complete_new_input.stdout, b"new input\n");
    assert!(String::from_utf8_lossy(&complete_new_input.stderr).contains("cursor: null>"));

    let older_remainder = invoke(&["-c", "last", "--all"]);
    assert!(older_remainder.status.success());
    assert_eq!(older_remainder.stdout, remainder);
}

#[test]
fn stable_reuse_across_desks_keeps_each_desks_recovery_intact() {
    // Regression for the pre-release audit's finding 1, reproducing its
    // exact table: under stable minting, desk B paging the same content
    // as desk A REUSES A's record (first-writer desk retained) — and in
    // the broken version, B then had no `-c last` recovery while B's
    // touch also hijacked A's selection. Per-desk recency files fix
    // both: each desk recovers its OWN most recent use.
    let state = scratch_dir("stabledesk");
    let desk_a = scratch_dir("stabledesk-a");
    let desk_b = scratch_dir("stabledesk-b");
    let t_doc = desk_a.join("t.txt");
    let u_doc = desk_a.join("u.txt");
    std::fs::write(&t_doc, b"T1\nT2\nT3\nT4\n").unwrap();
    std::fs::write(&u_doc, b"U1\nU2\nU3\nU4\n").unwrap();

    let page = |dir: &PathBuf, args: &[&str]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(args)
            .current_dir(dir)
            .env("MOREOVER_STATE_DIR", &state)
            .env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output().unwrap()
    };

    assert!(page(&desk_a, &[t_doc.to_str().unwrap(), "-2"]).status.success());
    assert!(page(&desk_a, &[u_doc.to_str().unwrap(), "-2"]).status.success());
    // B pages the same T content: stable minting reuses A's record
    assert!(page(&desk_b, &[t_doc.to_str().unwrap(), "-2"]).status.success());

    // B recovers the cursor it was just handed (the audit's failing row)
    let b_last = page(&desk_b, &["-c", "last", "--all"]);
    assert!(b_last.status.success(), "{}", String::from_utf8_lossy(&b_last.stderr));
    assert_eq!(b_last.stdout, b"T3\nT4\n");

    // and A's own recovery is undisturbed by B's activity: A's most
    // recent use is still U
    let a_last = page(&desk_a, &["-c", "last", "--all"]);
    assert!(a_last.status.success(), "{}", String::from_utf8_lossy(&a_last.stderr));
    assert_eq!(a_last.stdout, b"U3\nU4\n");
}

#[test]
#[cfg(unix)]
fn a_failed_recovery_update_is_an_error_naming_the_minted_cursor() {
    // Re-check finding 1: the desks/ update was best-effort — with
    // desks/ unwritable, paging still exited 0 while `-c last` silently
    // pointed at an older stream. The failure now propagates, and the
    // diagnostic names the minted id the reader would otherwise lose.
    use std::os::unix::fs::PermissionsExt;
    let state = scratch_dir("recfail");
    let desk = scratch_dir("recfail-desk");
    let t = desk.join("t.txt");
    std::fs::write(&t, lines(4)).unwrap();

    let page = |args: &[&str]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(args)
            .current_dir(&desk)
            .env("MOREOVER_STATE_DIR", &state)
            .env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output().unwrap()
    };
    assert!(page(&[t.to_str().unwrap(), "-2"]).status.success());

    let desks = state.join("desks");
    std::fs::set_permissions(&desks, std::fs::Permissions::from_mode(0o500)).unwrap();
    let out = page(&[t.to_str().unwrap(), "-2"]);
    std::fs::set_permissions(&desks, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(!out.status.success(), "an unrecordable recovery update must not report success");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("was minted"), "the diagnostic must hand back the minted id: {err}");
    assert!(err.contains("recovery record"), "got: {err}");
}

#[test]
fn a_damaged_recovery_record_reports_instead_of_selecting_an_older_stream() {
    // Re-check finding 1, read side: present-but-bad recovery state is
    // damage, not legacy absence — `-c last` must say so, never silently
    // hand back an older stream.
    let state = scratch_dir("recdamage");
    let desk = scratch_dir("recdamage-desk");
    let t = desk.join("t.txt");
    std::fs::write(&t, lines(4)).unwrap();
    let page = |args: &[&str]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(args)
            .current_dir(&desk)
            .env("MOREOVER_STATE_DIR", &state)
            .env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output().unwrap()
    };
    assert!(page(&[t.to_str().unwrap(), "-2"]).status.success());

    // corrupt this desk's recovery record in place
    let rec = std::fs::read_dir(state.join("desks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .unwrap();
    std::fs::write(&rec, b"garbage\n").unwrap();

    let out = page(&["-c", "last", "--all"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("malformed"), "damage must be reported, not scanned around: {err}");
}

#[test]
fn a_recovery_record_naming_a_foreign_desk_is_damage_not_a_collision() {
    // The confirmation pass's release condition: a stored `desk=` that
    // differs from the caller's was classified as a hash collision and
    // fell through to the legacy mtime scan — but the code never checked
    // whether that directory could actually belong at this recovery
    // filename. An edited or misplaced record (desk that does NOT hash
    // here) is DETECTABLE damage: the scan could silently select an
    // older stream. Reject it; only a stored desk that genuinely hashes
    // to the same filename may fall through.
    let state = scratch_dir("foreigndesk");
    let desk = scratch_dir("foreigndesk-desk");
    let t = desk.join("t.txt");
    let u = desk.join("u.txt");
    std::fs::write(&t, b"T1\nT2\nT3\nT4\n").unwrap();
    std::fs::write(&u, b"U1\nU2\nU3\nU4\n").unwrap();
    let page = |args: &[&str]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
        cmd.args(args)
            .current_dir(&desk)
            .env("MOREOVER_STATE_DIR", &state)
            .env_remove("MOREOVER_TRAILER")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output().unwrap()
    };
    // the confirmation's sequence: T, then U, then a stable reuse of T
    assert!(page(&[t.to_str().unwrap(), "-2"]).status.success());
    assert!(page(&[u.to_str().unwrap(), "-2"]).status.success());
    assert!(page(&[t.to_str().unwrap(), "-2"]).status.success());

    // rewrite the recovery record's desk to a directory that cannot
    // hash to this filename, keeping the id it points at intact
    let rec = std::fs::read_dir(state.join("desks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .unwrap();
    let text = std::fs::read_to_string(&rec).unwrap();
    let forged: String = text
        .lines()
        .map(|l| {
            if l.starts_with("desk=") {
                "desk=/somewhere/else/entirely".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&rec, forged + "\n").unwrap();

    let out = page(&["-c", "last", "--all"]);
    assert_eq!(out.status.code(), Some(1), "detectable damage must be an error, not a scan");
    assert!(out.stdout.is_empty(), "no page content may accompany the damage error");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("malformed"), "the damage diagnostic must be reported: {err}");
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

/// Pull the cursor id out of a trailer on stderr (`cursor: Xxxx>`).
fn cursor_in(stderr: &[u8]) -> String {
    let s = String::from_utf8_lossy(stderr);
    let tail = s.split("cursor: ").nth(1).expect("trailer with a cursor");
    tail.split('>').next().unwrap().trim().to_string()
}

/// Build a desk-aware invocation (cwd matters to `ls` and `-c last`).
fn at_desk(desk: &PathBuf, state: &PathBuf, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moreover"));
    cmd.args(args)
        .current_dir(desk)
        .env("MOREOVER_STATE_DIR", state)
        .env_remove("MOREOVER_TRAILER")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.output().unwrap()
}

#[test]
fn ls_shows_this_desks_cursors_and_marks_recovery() {
    // The first-contact field report's ask, and the audit's table shape:
    // a reader asking "what streams do I have parked HERE?" — including
    // the stable-reuse case where this desk's newest cursor was minted
    // from another directory (`ls` must not hide what `-c last` selects).
    let state = scratch_dir("ls");
    let desk_a = scratch_dir("ls-a");
    let desk_b = scratch_dir("ls-b");
    let t = desk_a.join("t.txt");
    std::fs::write(&t, b"T1\nT2\nT3\nT4\n").unwrap();

    let paged = at_desk(&desk_a, &state, &[t.to_str().unwrap(), "-2"]);
    assert!(paged.status.success());
    let t_id = cursor_in(&paged.stderr);

    // A sees its cursor, marked as what `-c last` would select
    let ls_a = at_desk(&desk_a, &state, &["ls"]);
    assert!(ls_a.status.success(), "{}", String::from_utf8_lossy(&ls_a.stderr));
    let listing = String::from_utf8(ls_a.stdout).unwrap();
    assert!(listing.contains(&t_id), "listing must name the cursor: {listing}");
    assert!(listing.contains("<- last"), "the recovery selection must be marked: {listing}");
    // a cursor names the NEXT position: after one 2-line page of a
    // 4-line input, the parked position is page 2, 2 lines consumed
    assert!(listing.contains("page 2"), "position belongs in the listing: {listing}");
    assert!(listing.contains("line 2/4"), "consumed/total belongs in the listing: {listing}");

    // B has nothing yet — explicit emptiness, exit 0
    let ls_b = at_desk(&desk_b, &state, &["ls"]);
    assert!(ls_b.status.success());
    assert!(String::from_utf8_lossy(&ls_b.stdout).contains("no cursors on this desk"));

    // B pages the same content: stable minting reuses A's record; B's ls
    // must still show B's recovery selection, annotated as reused
    assert!(at_desk(&desk_b, &state, &[t.to_str().unwrap(), "-2"]).status.success());
    let ls_b2 = at_desk(&desk_b, &state, &["ls"]);
    let listing = String::from_utf8(ls_b2.stdout).unwrap();
    assert!(listing.contains(&t_id), "the reused cursor must appear: {listing}");
    assert!(listing.contains("reused here"), "the cross-desk note must appear: {listing}");

    // --everywhere sees it once, with its minting desk
    let everywhere = at_desk(&desk_b, &state, &["ls", "--everywhere"]);
    let listing = String::from_utf8(everywhere.stdout).unwrap();
    assert!(listing.contains(&t_id));
    assert!(listing.contains(desk_a.to_str().unwrap()), "desk column: {listing}");
}

#[test]
fn drop_retires_a_cursor_its_spool_and_its_recovery_record() {
    // ADR-0003's gloss: "declare a parked stream finished." After drop,
    // the id must stop resolving, the saved input must be freed when
    // nothing else references it, and `-c last` must not resurrect it.
    let state = scratch_dir("drop");
    let desk = scratch_dir("drop-desk");
    let t = desk.join("t.txt");
    std::fs::write(&t, b"T1\nT2\nT3\nT4\n").unwrap();

    let paged = at_desk(&desk, &state, &[t.to_str().unwrap(), "-2"]);
    let t_id = cursor_in(&paged.stderr);

    let dropped = at_desk(&desk, &state, &["drop", &t_id]);
    assert!(dropped.status.success(), "{}", String::from_utf8_lossy(&dropped.stderr));
    let said = String::from_utf8(dropped.stdout).unwrap();
    assert!(said.contains(&t_id), "the report names what was dropped: {said}");

    let resumed = at_desk(&desk, &state, &["-c", &t_id, "--all"]);
    assert_eq!(resumed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&resumed.stderr).contains("unknown cursor"));

    let last = at_desk(&desk, &state, &["-c", "last", "--all"]);
    assert_eq!(last.status.code(), Some(1), "recovery must not resurrect a dropped stream");
    assert!(String::from_utf8_lossy(&last.stderr).contains("no cursors were minted"));

    let spools: Vec<_> = std::fs::read_dir(state.join("spools"))
        .map(|d| d.flatten().collect())
        .unwrap_or_default();
    assert!(spools.is_empty(), "an unreferenced spool must be freed");

    // dropping an unknown id teaches the same rule resume does
    let unknown = at_desk(&desk, &state, &["drop", "zz9q"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("only valid if moreover printed it"));
}

#[test]
#[cfg(unix)]
fn gc_sweeps_by_age_and_spares_the_young() {
    // `gc DAYS` is a promise about AGE: an old parked stream goes, a
    // young one stays resumable, and the sweep's own report says what
    // happened. Backdating uses touch(1), as a shell operator would.
    let state = scratch_dir("gc");
    let desk = scratch_dir("gc-desk");
    let t = desk.join("t.txt");
    let u = desk.join("u.txt");
    std::fs::write(&t, b"T1\nT2\nT3\nT4\n").unwrap();
    std::fs::write(&u, b"U1\nU2\nU3\nU4\n").unwrap();

    let old = cursor_in(&at_desk(&desk, &state, &[t.to_str().unwrap(), "-2"]).stderr);
    let young = cursor_in(&at_desk(&desk, &state, &[u.to_str().unwrap(), "-2"]).stderr);

    // age the first record past the default window
    let aged = Command::new("touch")
        .args(["-m", "-t", "202501010000"])
        .arg(state.join("cursors").join(&old))
        .status()
        .unwrap();
    assert!(aged.success());

    let swept = at_desk(&desk, &state, &["gc"]);
    assert!(swept.status.success(), "{}", String::from_utf8_lossy(&swept.stderr));
    let report = String::from_utf8(swept.stdout).unwrap();
    assert!(report.contains("removed 1 record"), "the report counts the sweep: {report}");

    assert_eq!(at_desk(&desk, &state, &["-c", &old, "--all"]).status.code(), Some(1));
    let resumed = at_desk(&desk, &state, &["-c", &young, "--all"]);
    assert!(resumed.status.success(), "a young cursor must survive gc");
    assert_eq!(resumed.stdout, b"U3\nU4\n");

    // gc 0 declares the whole desk finished
    assert!(at_desk(&desk, &state, &["gc", "0"]).status.success());
    let ls = at_desk(&desk, &state, &["ls"]);
    assert!(String::from_utf8_lossy(&ls.stdout).contains("no cursors on this desk"));
}

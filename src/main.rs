// moreover — a pager for readers who can't press space.
//
// The binary is a thin shell over the library: parse the surface ruled in
// ADR-0002/0003, wire the streams, route the trailer (stderr by default —
// stdout stays pure content, so moreover composes in pipelines).
// Flag names await Mint 5's naming review (ADR-0003 iteration 4); they are
// centralized here and in USAGE so strikes cost string edits only.
// Design record: docs/adr/. The name's full story: docs/adr/0001.

use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use moreover::chunker::Unit;
use moreover::paging::{page_new, page_resume, Take};
use moreover::store::{FsStore, MintMode};
use moreover::trailer::{lookup, render_with};

const USAGE: &str = "\
moreover — a pager for readers who can't press space

Usage:
  <producer> | moreover [-N | -n N | --bytes N | --all]     (pipe mode)
  moreover FILE [-N | -n N | --bytes N | --all]             (file mode)
  moreover -c CURSOR [-N | --all] [--overlap N]             (resume; no input needed)

Paging:
  -N, -n N, --lines N   page size in lines (default: 10)
  --bytes N             page size in bytes
  --all                 everything (remaining)
  --overlap N           on resume, reprint the last N units before the new
                        page (context re-anchoring; a no-op on first pages)

Resumption:
  -c, --cursor ID       resume the stream that ID names
                        (a cursor is only valid if moreover printed it —
                        never invent or extrapolate one)
  -c last               select the newest saved cursor for this working
                        directory in the selected state directory
  --mint stable|fresh   cursor-id policy, asserted (default: stable —
                        the same resume repeated yields the same next
                        cursor; fresh mints a new id every time)

State:
  --state-dir PATH      spool/cursor store (default: $MOREOVER_STATE_DIR,
                        else $XDG_STATE_HOME/moreover, else ~/.local/state/moreover)

Trailer (the v0 grammar is a compatibility promise):
  --trailer DEST        route the trailer: stderr | stdout |
                        none | fd:N | file:PATH (append)
                        This flag overrides MOREOVER_TRAILER for this call.
                        Without this flag, MOREOVER_TRAILER sets the
                        default; unset or empty uses stderr.
  --schema NAME         trailer schema (default: v0)
  --schema-show         print the active schema's templates and exit
  --schema-template T   render the trailer with template T instead

Desk (subcommands — the standalone world; they never appear in pipes):
  moreover contract     print the model-facing contract
  moreover ls           list this directory's cursors (--everywhere: all)
  moreover drop CURSOR  declare a parked stream finished; free what it held
  moreover gc [DAYS]    sweep cursors unused for DAYS days (default: 7;
                        0 sweeps everything); orphaned input follows
  (stat: reserved for the desk, not yet available)
  Desk verbs accept --state-dir like the pipe world does.

Introspection:
  --help                this text
  --version             version
";

/// The desk world (ADR-0003 QST-SUBCOMMAND-GRAMMAR, accepted): bare
/// `moreover` + flags pages; subcommands do desk operations. All five
/// verbs are reserved from day one so a file named `ls` can never
/// silently page — `./ls` pages it.
const DESK_VERBS: [&str; 5] = ["contract", "ls", "stat", "drop", "gc"];

/// The null call (first dogfooding seed, 2026-09-23): bare `moreover` on
/// a terminal has no input coming — cat/head/tail hang here; less/more
/// check. We check, and guide both audiences instead (the audience words
/// are the naming authority's strike). Piped-but-EMPTY input is not a
/// null call: an empty stream still earns its 0/0 trailer — the
/// inductive base case.
const NULL_CALL_GUIDE: &str = "\
moreover — a pager for readers who can't press space

Give moreover a file or pipe it some input. It saves that input, prints
one page, and hands back a cursor, so a later invocation — even from a
fresh shell — resumes exactly where this one stopped.

Documentation, for humans:  moreover --help
Contract, for models:       moreover contract";

/// The machine-facing contract (`moreover contract`, ADR-0003
/// QST-SELF-DESCRIPTION + QST-SUBCOMMAND-GRAMMAR). Describes the current
/// implementation; the trailer templates remain the frozen v0 grammar.
const CONTRACT: &str = "\
moreover: contract (v0)

purpose:
  Save input from a pipe or file, print a page, and resume the saved input
  in a later invocation using a printed cursor.
  Input must finish before the first page appears: the whole input is
  read into memory and saved on disk. Unbounded input never reaches a page.

invocations:
  stdin:    <producer> | moreover -10
  file:     moreover FILE -10
  resume:   moreover -c CURSOR --all
  recover:  moreover -c last
  contract: moreover contract
  list:     moreover ls
  drop:     moreover drop CURSOR
  sweep:    moreover gc [DAYS]
  Resume reads saved input, ignores stdin, and rejects an input file.
  Subcommands never read stdin. stat is reserved, not yet available.
  Prefix a filename matching a subcommand with ./ (for example, ./ls).

desk verbs:
  ls lists this working directory's saved cursors, one per line, newest
  first: ID, page, line (with the saved input's total when known), mint
  mode, the saved input's name, and age. The line -c last would select
  is marked. A cursor minted from another directory but last used here
  is listed with that note. ls --everywhere lists every directory's
  cursors with their directories. Listing changes nothing.
  drop CURSOR removes that cursor's record. Saved input still referenced
  by another cursor is kept; otherwise it is freed. Recovery records
  naming the dropped cursor are cleared, so -c last there selects an
  older record or reports none. Other cursors are unaffected.
  gc DAYS removes every cursor record unused for more than DAYS days
  (default 7; gc 0 removes all), then frees saved input and recovery
  records nothing references. Unreadable record debris is swept too.
  When a removed record occupies a slot that a kept stable ID's
  derivation walked past, an empty placeholder file is left in its
  place so that stable IDs stay reproducible; placeholders are swept
  once nothing depends on them. drop and gc are permanent: a removed
  cursor ID stops resolving, and freed input cannot be resumed.
  Desk verbs accept --state-dir and never print a trailer.

paging:
  Every invocation defaults to 10 lines, including resume.
  -N, -n N, or --lines N selects lines; --bytes N selects bytes.
  --all prints the remainder with line counts; use it without a page size.
  Byte pages can split lines and encoded characters. Changing units on
  resume keeps the saved byte position, which may be inside a line.
  --overlap N repeats up to N preceding units before a resumed page,
  using this invocation's unit. It has no effect on the first page.

trailer (schema v0; a compatibility promise):
  paged form:  <moreover: page {page}, {shown}/{total} {unit}, cursor: {cursor}>
  --all form:  <moreover: {shown}/{total} {unit}, cursor: {cursor}>
  {shown} counts units from the start of saved input through this page's
  end; {total} counts the whole saved input. Both use this invocation's
  unit (lines or bytes). Overlap is not added again. Totals are numeric.
  {page} starts at 1 and advances through successive cursors.
  cursor: null means no input remains after this page; do not resume null.
  Otherwise, use the printed cursor to start the next page.
  --schema v0 selects the only current schema; --schema-show prints it.
  --schema-template T replaces the template using the placeholders above.

output:
  Content goes to stdout.
  --trailer DEST accepts stderr, stdout, none, fd:N, or file:PATH.
  Destination precedence: --trailer > MOREOVER_TRAILER > stderr.
  MOREOVER_TRAILER accepts the same destinations and sets the default
  for invocations that inherit it. Unset or empty uses stderr.
  An explicit --trailer overrides even an invalid environment value.
  If used as the default, an unrecognized environment destination causes
  a usage error before any content is emitted.
  The --help, --version, and --schema-show flags and the contract
  subcommand ignore MOREOVER_TRAILER.
  stdout places the trailer after the content; none suppresses it;
  fd:N writes to an inherited descriptor; file:PATH appends to a file.

shell redirection (MOREOVER_TRAILER unset; no --trailer flag):
  moreover FILE -10
    Content goes to stdout; the trailer goes to stderr. Capture both.
  moreover FILE -10 2>&1
    Content, trailer, and errors share stdout. A downstream pipe or
    captured value receives the trailer as data alongside the content.
  moreover FILE -10 2>/dev/null
    The trailer and errors are discarded. A cursor may still be saved,
    but its ID is lost from this output.
  For callers that capture only stdout, use --trailer stdout to include
  the trailer after the content. This still mixes content and metadata;
  use --trailer fd:N or file:PATH when they need separate destinations.
  Changing --trailer does not redirect errors; they still use stderr.
  Check the exit status before using the output.

cursors:
  Use a printed cursor ID — never invent or extrapolate one.
  The reserved word last selects a saved cursor as described below.
  A cursor fixes a position in saved input, not page size, unit, or overlap.
  Resuming a printed cursor ID with the same page size, unit, and overlap
  repeats the same content. The next cursor ID may differ.
  Resumption leaves the original cursor unchanged.
  IDs are case-insensitive; o folds to 0, and i and l fold to 1.
  IDs start at four characters and can be longer.
  --mint stable (the default) and --mint fresh assert the ID policy;
  each is an assertion, valid under any default. Under stable, the next
  cursor ID is determined by the saved input, the byte position, and
  the page number, so repeating the same resume returns the same ID;
  a matching record made under any mode is reused. Under fresh, every
  resume mints a new ID. mode= in a record names its creation mode.
  No mode creates a successor when the trailer says cursor: null.

last (recovery):
  -c last selects this working directory's most recently used cursor,
  from a per-directory recovery record in the state directory. Records
  from before this mechanism are matched by their stored directory
  instead. No matching record is an error; a damaged recovery record is
  reported as an error rather than silently selecting an older stream.
  last is resolved again on each call. Other invocations in the same
  working directory and state directory can change its selection,
  including to another stream. Use a printed ID for a fixed position.
  A call that creates no cursor leaves last unchanged, even at exhaustion.
  Stop at cursor: null; another -c last can repeat already-read content.
  Older records without a working directory do not match last;
  they can still be resumed by their printed IDs.

state (first applicable entry wins):
  --state-dir PATH > $MOREOVER_STATE_DIR > $XDG_STATE_HOME/moreover
  > ~/.local/state/moreover
  A cursor requires its record and saved input in the selected directory.
  Keep that state to resume. The original file or producer is not reread.
  Reaching the end does not delete saved state.

exit codes:
  0 success; 1 reported I/O, state, or cursor error; 2 usage or schema error.
  Error messages go to stderr.
";

#[derive(Debug, Clone, PartialEq)]
enum TrailerDest {
    Stderr,
    Stdout,
    None,
    Fd(u32),
    File(PathBuf),
}

fn parse_trailer_dest(s: &str) -> Result<TrailerDest, String> {
    match s {
        "stderr" => Ok(TrailerDest::Stderr),
        "stdout" => Ok(TrailerDest::Stdout),
        "none" => Ok(TrailerDest::None),
        _ => {
            if let Some(n) = s.strip_prefix("fd:") {
                n.parse().map(TrailerDest::Fd).map_err(|_| format!("bad fd in --trailer {s}"))
            } else if let Some(p) = s.strip_prefix("file:") {
                Ok(TrailerDest::File(PathBuf::from(p)))
            } else {
                Err(format!(
                    "unknown --trailer destination: {s} (known: stderr, stdout, none, fd:N, file:PATH)"
                ))
            }
        }
    }
}

struct Args {
    take: Take,
    unit: Unit,
    cursor: Option<String>,
    input_file: Option<PathBuf>,
    overlap: usize,
    mint: MintMode,
    trailer_dest: Option<TrailerDest>,
    state_dir: Option<PathBuf>,
    schema: String,
    schema_show: bool,
    schema_template: Option<String>,
}

enum Parsed {
    Run(Box<Args>),
    Help,
    Version,
    Contract,
    Ls { everywhere: bool, state_dir: Option<PathBuf> },
    Gc { days: u64, state_dir: Option<PathBuf> },
    Drop { cursor: String, state_dir: Option<PathBuf> },
}

/// Parse a desk verb's tail: `--state-dir` is common to all three;
/// anything else is handed back to the verb for its own vocabulary.
fn desk_tail(rest: &[String]) -> Result<(Vec<String>, Option<PathBuf>), String> {
    let mut own = Vec::new();
    let mut state_dir = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "--state-dir" {
            let v = it.next().ok_or_else(|| "--state-dir needs a value".to_string())?;
            state_dir = Some(PathBuf::from(v));
        } else {
            own.push(a.clone());
        }
    }
    Ok((own, state_dir))
}

fn parse_args(argv: &[String]) -> Result<Parsed, String> {
    // Desk world: a subcommand must be the first token (git/cargo shape).
    if let Some(first) = argv.first() {
        match first.as_str() {
            "contract" => {
                if argv.len() > 1 {
                    return Err("moreover contract takes no arguments".to_string());
                }
                return Ok(Parsed::Contract);
            }
            "ls" => {
                let (own, state_dir) = desk_tail(&argv[1..])?;
                let mut everywhere = false;
                for a in &own {
                    match a.as_str() {
                        "--everywhere" => everywhere = true,
                        other => {
                            return Err(format!(
                                "moreover ls takes --everywhere and --state-dir only, got: {other}"
                            ))
                        }
                    }
                }
                return Ok(Parsed::Ls { everywhere, state_dir });
            }
            "gc" => {
                let (own, state_dir) = desk_tail(&argv[1..])?;
                let mut days: u64 = 7;
                match own.as_slice() {
                    [] => {}
                    [d] => {
                        days = d.parse().map_err(|_| {
                            format!("moreover gc takes an age in whole days, got: {d}")
                        })?;
                    }
                    _ => return Err("moreover gc takes at most one argument (DAYS)".to_string()),
                }
                return Ok(Parsed::Gc { days, state_dir });
            }
            "drop" => {
                let (own, state_dir) = desk_tail(&argv[1..])?;
                match own.as_slice() {
                    [id] => return Ok(Parsed::Drop { cursor: id.clone(), state_dir }),
                    _ => return Err("moreover drop takes exactly one cursor ID".to_string()),
                }
            }
            _ => {}
        }
        if DESK_VERBS.contains(&first.as_str()) {
            return Err(format!(
                "moreover {first} is a reserved desk subcommand (ADR-0003), not yet available; \
                 to page a file named '{first}', write ./{first}"
            ));
        }
    }
    let mut args = Args {
        take: Take::Units(10),
        unit: Unit::Lines,
        cursor: None,
        input_file: None,
        overlap: 0,
        mint: MintMode::Stable,
        trailer_dest: None,
        state_dir: None,
        schema: "v0".to_string(),
        schema_show: false,
        schema_template: None,
    };
    let mut it = argv.iter();
    let mut sized = false;
    while let Some(a) = it.next() {
        let mut want = |flag: &str| -> Result<String, String> {
            it.next().cloned().ok_or_else(|| format!("{flag} needs a value"))
        };
        match a.as_str() {
            "--help" => return Ok(Parsed::Help),
            "--version" => return Ok(Parsed::Version),
            "--all" => args.take = Take::All,
            "-n" | "--lines" => {
                let n = want(a)?.parse().map_err(|_| format!("bad count for {a}"))?;
                args.take = Take::Units(n);
                args.unit = Unit::Lines;
                sized = true;
            }
            "--bytes" => {
                let n = want(a)?.parse().map_err(|_| format!("bad count for {a}"))?;
                args.take = Take::Units(n);
                args.unit = Unit::Bytes;
                sized = true;
            }
            "--overlap" => {
                args.overlap = want(a)?.parse().map_err(|_| format!("bad count for {a}"))?;
            }
            "-c" | "--cursor" => args.cursor = Some(want(a)?),
            "--mint" => {
                args.mint = match want(a)?.as_str() {
                    "stable" => MintMode::Stable,
                    "fresh" => MintMode::Fresh,
                    other => return Err(format!("unknown --mint mode: {other} (known: stable, fresh)")),
                }
            }
            "--trailer" => args.trailer_dest = Some(parse_trailer_dest(&want(a)?)?),
            "--state-dir" => args.state_dir = Some(PathBuf::from(want(a)?)),
            "--schema" => args.schema = want(a)?,
            "--schema-show" => args.schema_show = true,
            "--schema-template" => args.schema_template = Some(want(a)?),
            s => {
                // head-style leading count: `moreover -10`
                if let Some(n) = s.strip_prefix('-').and_then(|d| d.parse::<usize>().ok()) {
                    args.take = Take::Units(n);
                    args.unit = Unit::Lines;
                    sized = true;
                } else if !s.starts_with('-') {
                    if args.input_file.is_some() {
                        return Err(format!("only one input file, got a second: {s}"));
                    }
                    args.input_file = Some(PathBuf::from(s));
                } else {
                    return Err(format!("unknown argument: {s}\n\n{USAGE}"));
                }
            }
        }
    }
    if matches!(args.take, Take::All) && sized {
        return Err("choose a page size or --all, not both".to_string());
    }
    if args.cursor.is_some() && args.input_file.is_some() {
        return Err("resume (-c) takes no input file: the cursor already names the stream".to_string());
    }
    Ok(Parsed::Run(Box::new(args)))
}

fn emit_trailer(dest: &TrailerDest, line: &str) -> io::Result<()> {
    match dest {
        TrailerDest::Stderr => eprintln!("{line}"),
        TrailerDest::Stdout => println!("{line}"),
        TrailerDest::None => {}
        TrailerDest::Fd(n) => {
            // /dev/fd/N reaches an inherited descriptor without unsafe fd
            // adoption (which would close the caller's fd on drop).
            let mut f = fs::OpenOptions::new().append(true).open(format!("/dev/fd/{n}"))?;
            writeln!(f, "{line}")?;
        }
        TrailerDest::File(p) => {
            let mut f = fs::OpenOptions::new().append(true).create(true).open(p)?;
            writeln!(f, "{line}")?;
        }
    }
    Ok(())
}

fn open_store(state_dir: Option<PathBuf>) -> Result<FsStore, (u8, String)> {
    let root = state_dir.unwrap_or_else(FsStore::default_root);
    FsStore::open(root.clone())
        .map_err(|e| (1, format!("cannot open state dir {}: {e}", root.display())))
}

fn current_desk() -> String {
    std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()
}

/// Humanized age for `ls` — coarse on purpose: a reader deciding what to
/// resume or sweep needs magnitude, not timestamps.
fn age_of(t: std::time::SystemTime) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(t)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match secs {
        0..=59 => "under a minute ago".to_string(),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86_399 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86_400),
    }
}

fn ls_line(store: &FsStore, e: &moreover::store::CursorEntry) -> String {
    use moreover::store::Store;
    let c = e.cursor.as_ref().expect("ls only prints parseable records");
    let total = store
        .spool_meta(&c.spool)
        .ok()
        .flatten()
        .map(|m| format!("/{}", m.total_lines()))
        .unwrap_or_default();
    let mode = e.mode.as_deref().unwrap_or("fresh");
    format!(
        "{}  page {}  line {}{}  {}  spool {}  {}",
        e.id, c.page, c.line, total, mode, c.spool, age_of(e.modified)
    )
}

fn run_ls(everywhere: bool, state_dir: Option<PathBuf>) -> Result<(), (u8, String)> {
    let store = open_store(state_dir)?;
    let entries = store
        .list_cursors()
        .map_err(|e| (1, format!("cannot list cursors: {e}")))?;
    let records: Vec<_> = entries.iter().filter(|e| e.cursor.is_some()).collect();

    if everywhere {
        if records.is_empty() {
            println!("no cursors in {}", store.root().display());
            return Ok(());
        }
        for e in &records {
            let desk = e.cursor.as_ref().map(|c| c.desk.as_str()).unwrap_or("");
            let desk = if desk.is_empty() { "(no desk recorded)" } else { desk };
            println!("{}  desk {}", ls_line(&store, e), desk);
        }
        return Ok(());
    }

    let desk = current_desk();
    // This desk's view: records minted here, plus — under stable reuse —
    // a record another desk minted first but this desk used last (the
    // per-desk recovery file knows; the shared record keeps its first
    // writer's desk). A damaged recovery record is surfaced, not fatal:
    // the listing still stands on the records themselves.
    let recovery = match store.last_cursor_for_desk(&desk) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("note: this desk's recovery record is unusable ({e})");
            None
        }
    };
    let mine: Vec<_> = records
        .iter()
        .filter(|e| e.cursor.as_ref().is_some_and(|c| c.desk == desk))
        .collect();
    let mut shown = false;
    for e in &mine {
        let mark = if Some(e.id.as_str()) == recovery.as_deref() { "  <- last" } else { "" };
        println!("{}{}", ls_line(&store, e), mark);
        shown = true;
    }
    if let Some(id) = &recovery {
        if !mine.iter().any(|e| &e.id == id) {
            if let Some(e) = records.iter().find(|e| &e.id == id) {
                println!(
                    "{}  <- last (minted from another directory; reused here)",
                    ls_line(&store, e)
                );
                shown = true;
            }
        }
    }
    if !shown {
        println!("no cursors on this desk ({desk})");
    }
    Ok(())
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

fn run_gc(days: u64, state_dir: Option<PathBuf>) -> Result<(), (u8, String)> {
    let store = open_store(state_dir)?;
    let report = store.gc(days).map_err(|e| (1, format!("gc failed: {e}")))?;
    println!(
        "kept {}; removed {}; {} kept stable IDs reproducible; freed {}; cleared {}",
        plural(report.kept, "cursor"),
        plural(report.removed, "record"),
        plural(report.tombstoned, "placeholder"),
        plural(report.spools_freed, "spool"),
        plural(report.desks_cleared, "recovery record"),
    );
    Ok(())
}

fn run_drop(cursor: &str, state_dir: Option<PathBuf>) -> Result<(), (u8, String)> {
    let store = open_store(state_dir)?;
    let report = store.drop_cursor(cursor).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            (1, format!(
                "unknown cursor '{cursor}' (state: {}) — a cursor is only valid if \
                 moreover printed it; run `moreover contract` for the rules",
                store.root().display()
            ))
        } else {
            (1, format!("cannot drop '{cursor}': {e}"))
        }
    })?;
    let fate = if report.tombstoned {
        "an empty placeholder remains: a live stable ID depends on this slot"
    } else {
        "record removed"
    };
    println!(
        "dropped {cursor} ({fate}); freed {}; cleared {}",
        plural(report.spools_freed, "spool"),
        plural(report.desks_cleared, "recovery record"),
    );
    Ok(())
}

fn run() -> Result<(), (u8, String)> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv).map_err(|msg| (2, msg))? {
        Parsed::Help => {
            print!("{USAGE}");
            return Ok(());
        }
        Parsed::Version => {
            println!("moreover {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Parsed::Contract => {
            print!("{CONTRACT}");
            return Ok(());
        }
        Parsed::Ls { everywhere, state_dir } => return run_ls(everywhere, state_dir),
        Parsed::Gc { days, state_dir } => return run_gc(days, state_dir),
        Parsed::Drop { cursor, state_dir } => return run_drop(&cursor, state_dir),
        Parsed::Run(args) => args,
    };

    let schema = lookup(&args.schema)
        .ok_or_else(|| (2, format!("unknown schema: {} (known: v0)", args.schema)))?;

    if args.schema_show {
        println!("schema: {}", schema.name);
        println!("paged: {}", schema.paged);
        println!("all:   {}", schema.all);
        return Ok(());
    }

    // Trailer destination, resolved lazily per the accepted precedence
    // (flag > env > built-in stderr; ADR-0003 QST-ENV-OVERRIDE): the env
    // is consulted ONLY when no explicit flag was given, so an explicit
    // --trailer wins even over a typo'd MOREOVER_TRAILER, and help,
    // version, and contract never touch the variable at all. (Lector 6's
    // audit of 1df18dd caught the eager-parse mismatch.) An invalid value
    // that IS consulted errors loudly, before any content is emitted.
    let trailer_dest = match &args.trailer_dest {
        Some(d) => d.clone(),
        None => match std::env::var("MOREOVER_TRAILER") {
            Ok(v) if !v.is_empty() => parse_trailer_dest(&v)
                .map_err(|e| (2, format!("MOREOVER_TRAILER: {e}")))?,
            _ => TrailerDest::Stderr,
        },
    };

    let root = args.state_dir.clone().unwrap_or_else(FsStore::default_root);
    let store = FsStore::open(root.clone())
        .map_err(|e| (1, format!("cannot open state dir {}: {e}", root.display())))?;

    // `last` is reserved resume vocabulary (never mintable: ids always mix
    // letters and digits) — resolved against this directory's desk before
    // the id path, so it is never case-folded like a minted id would be.
    let cursor = match &args.cursor {
        Some(id) if id.eq_ignore_ascii_case("last") => {
            let desk = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            match store.last_cursor_for_desk(&desk) {
                Ok(Some(id)) => Some(id),
                Ok(None) => {
                    return Err((1, format!(
                        "no cursors were minted from this directory (desk: {desk}) — \
                         resume from the directory where you paged, or pass an explicit \
                         cursor; run `moreover contract` for the rules"
                    )));
                }
                Err(e) => return Err((1, format!("cannot resolve last: {e}"))),
            }
        }
        other => other.clone(),
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let trailer = match &cursor {
        Some(id) => page_resume(&store, id, args.take, args.unit, args.overlap, args.mint, &mut out)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    (1, format!("unknown cursor '{id}' (state: {}) — a cursor is only valid if moreover printed it; run `moreover contract` for the rules", root.display()))
                } else {
                    (1, format!("cannot resume '{id}': {e}"))
                }
            })?,
        None => {
            let input = match &args.input_file {
                Some(path) => fs::read(path)
                    .map_err(|e| (1, format!("cannot read {}: {e}", path.display())))?,
                None => {
                    if io::stdin().is_terminal() {
                        // The null call: no pipe, no file, no cursor —
                        // waiting would hang a reader who meant to ask.
                        return Err((2, NULL_CALL_GUIDE.to_string()));
                    }
                    let mut buf = Vec::new();
                    io::stdin()
                        .lock()
                        .read_to_end(&mut buf)
                        .map_err(|e| (1, format!("cannot read stdin: {e}")))?;
                    buf
                }
            };
            page_new(&store, &input, args.take, args.unit, args.mint, &mut out)
                .map_err(|e| (1, format!("cannot page: {e}")))?
        }
    };
    out.flush().map_err(|e| (1, format!("cannot flush stdout: {e}")))?;

    // The trailer is the interface; stderr is its default home by ruling,
    // and --trailer moves it for harnesses that capture only stdout.
    let line = render_with(schema, args.schema_template.as_deref(), &trailer);
    emit_trailer(&trailer_dest, &line).map_err(|e| (1, format!("cannot emit trailer: {e}")))?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, msg)) => {
            let _ = writeln!(io::stderr(), "{msg}");
            ExitCode::from(code)
        }
    }
}

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
use moreover::store::FsStore;
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

State:
  --state-dir PATH      spool/cursor store (default: $MOREOVER_STATE_DIR,
                        else $XDG_STATE_HOME/moreover, else ~/.local/state/moreover)

Trailer (the v0 grammar is a compatibility promise):
  --trailer DEST        route the trailer: stderr (default) | stdout |
                        none | fd:N | file:PATH (append)
  --schema NAME         trailer schema (default: v0)
  --schema-show         print the active schema's templates and exit
  --schema-template T   render the trailer with template T instead

Desk (subcommands — the standalone world; they never appear in pipes):
  moreover contract     print the model-facing contract
  (ls, stat, drop, gc: reserved for the desk, not yet available)

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
  contract: moreover contract
  Resume reads saved input, ignores stdin, and rejects an input file.
  The contract takes no arguments and does not read stdin.
  ls, stat, drop, and gc are reserved subcommands, not yet available.
  Prefix a filename matching a subcommand with ./ (for example, ./ls).

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
  Content goes to stdout. The trailer goes to stderr by default.
  --trailer DEST accepts stderr, stdout, none, fd:N, or file:PATH.
  stdout places the trailer after the content; none suppresses it;
  fd:N writes to an inherited descriptor; file:PATH appends to a file.

cursors:
  Only reuse a cursor moreover printed — never invent or extrapolate one.
  A cursor fixes a position in saved input, not page size, unit, or overlap.
  Resuming the same cursor with the same page size, unit, and overlap
  repeats the same content. The next cursor ID may differ.
  Resumption leaves the original cursor unchanged.
  IDs are case-insensitive; o folds to 0, and i and l fold to 1.

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
    trailer_dest: TrailerDest,
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
}

fn parse_args(argv: &[String]) -> Result<Parsed, String> {
    // Desk world: a subcommand must be the first token (git/cargo shape).
    if let Some(first) = argv.first() {
        if first == "contract" {
            if argv.len() > 1 {
                return Err("moreover contract takes no arguments".to_string());
            }
            return Ok(Parsed::Contract);
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
        trailer_dest: TrailerDest::Stderr,
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
            "--trailer" => args.trailer_dest = parse_trailer_dest(&want(a)?)?,
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

    let root = args.state_dir.clone().unwrap_or_else(FsStore::default_root);
    let store = FsStore::open(root.clone())
        .map_err(|e| (1, format!("cannot open state dir {}: {e}", root.display())))?;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let trailer = match &args.cursor {
        Some(id) => page_resume(&store, id, args.take, args.unit, args.overlap, &mut out)
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
            page_new(&store, &input, args.take, args.unit, &mut out)
                .map_err(|e| (1, format!("cannot page: {e}")))?
        }
    };
    out.flush().map_err(|e| (1, format!("cannot flush stdout: {e}")))?;

    // The trailer is the interface; stderr is its default home by ruling,
    // and --trailer moves it for harnesses that capture only stdout.
    let line = render_with(schema, args.schema_template.as_deref(), &trailer);
    emit_trailer(&args.trailer_dest, &line).map_err(|e| (1, format!("cannot emit trailer: {e}")))?;
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

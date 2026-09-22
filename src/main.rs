// moreover — a pager for readers who can't press space.
//
// The binary is a thin shell over the library: parse the surface ruled in
// ADR-0002/0003, wire the streams, route the trailer (stderr by default —
// stdout stays pure content, so moreover composes in pipelines).
// Flag names await Mint 5's naming review (ADR-0003 iteration 4); they are
// centralized here and in USAGE so strikes cost string edits only.
// Design record: docs/adr/. The name's full story: docs/adr/0001.

use std::fs;
use std::io::{self, Read, Write};
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

Introspection:
  --agent               print the machine-facing contract and exit
  --help                this text
  --version             version
";

/// The machine-facing contract (`--agent`, ADR-0003 QST-SELF-DESCRIPTION).
/// DRAFT text: the mechanism is stable; the words are under review by the
/// crew's public-language seat.
const AGENT_CONTRACT: &str = "\
moreover: agent contract (v0)

what: a non-interactive pager. It pages a stream now and lets you resume
it in a LATER process invocation via a cursor. The trailer line teaches
you the rest at the exact moment there is more.

trailer (schema v0; a compatibility promise):
  paged form:  <moreover: page {page}, {shown}/{total} {unit}, cursor: {cursor}>
  --all form:  <moreover: {shown}/{total} {unit}, cursor: {cursor}>
  cursor: null   means the stream is exhausted.
  {shown} is cumulative through this page; {total} may be ? if unknown.
  destination: stderr by default; --trailer stderr|stdout|none|fd:N|file:PATH

cursors:
  - a cursor names (stream, position); page size is never part of it
  - only reuse a cursor moreover printed — never invent or extrapolate one
  - read case-insensitively (o folds to 0; i and l fold to 1)
  - immutable: resuming the same cursor twice yields the same page
  - scope: this machine's state dir only; cursors do not travel

invocations:
  pipe mode:        <producer> | moreover -10
  file mode:        moreover FILE -10
  resume (no input): moreover -c CURSOR --all
  --overlap N on resume reprints the last N units first (re-anchoring;
  the trailer's numbers do not count the reprint)

units: lines (default) | bytes (--bytes N)
state: $MOREOVER_STATE_DIR > $XDG_STATE_HOME/moreover > ~/.local/state/moreover
exit codes: 0 ok · 1 stream/cursor error (message on stderr, written to
you, the reader) · 2 usage error
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
    Agent,
}

fn parse_args(argv: &[String]) -> Result<Parsed, String> {
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
            "--agent" => return Ok(Parsed::Agent),
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
        Parsed::Agent => {
            print!("{AGENT_CONTRACT}");
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
                    (1, format!("unknown cursor '{id}' (state: {}) — a cursor is only valid if moreover printed it", root.display()))
                } else {
                    (1, format!("cannot resume '{id}': {e}"))
                }
            })?,
        None => {
            let input = match &args.input_file {
                Some(path) => fs::read(path)
                    .map_err(|e| (1, format!("cannot read {}: {e}", path.display())))?,
                None => {
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

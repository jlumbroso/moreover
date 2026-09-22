// moreover — a pager for readers who can't press space.
//
// The binary is a thin shell over the library: parse the surface ruled in
// ADR-0002, wire stdin/stdout, and put the trailer on stderr (his ruling:
// stdout stays pure content, so moreover composes in pipelines).
// Design record: docs/adr/0002-pagination-for-a-reader-without-hands.md
// The name's full story: docs/adr/0001-the-name-moreover.md

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
  <producer> | moreover [-N | -n N | --bytes N | --all]
  moreover -c CURSOR [-N | -n N | --bytes N | --all]

Paging:
  -N, -n N, --lines N   page size in lines (default: 10)
  --bytes N             page size in bytes
  --all                 everything (remaining)
  -c, --cursor ID       resume the stream that ID names

State:
  --state-dir PATH      spool/cursor store (default: $MOREOVER_STATE_DIR,
                        else $XDG_STATE_HOME/moreover, else ~/.local/state/moreover)

Trailer (printed on stderr; the v0 grammar is a compatibility promise):
  --schema NAME         trailer schema (default: v0)
  --schema-show         print the active schema's templates and exit
  --schema-template T   render the trailer with template T instead

  --help                this text
  --version             version
";

struct Args {
    take: Take,
    unit: Unit,
    cursor: Option<String>,
    state_dir: Option<PathBuf>,
    schema: String,
    schema_show: bool,
    schema_template: Option<String>,
}

enum Parsed {
    Run(Box<Args>),
    Help,
    Version,
}

fn parse_args(argv: &[String]) -> Result<Parsed, String> {
    let mut args = Args {
        take: Take::Units(10),
        unit: Unit::Lines,
        cursor: None,
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
            "-c" | "--cursor" => args.cursor = Some(want(a)?),
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
                } else {
                    return Err(format!("unknown argument: {s}\n\n{USAGE}"));
                }
            }
        }
    }
    if matches!(args.take, Take::All) && sized {
        return Err("choose a page size or --all, not both".to_string());
    }
    Ok(Parsed::Run(Box::new(args)))
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
        Some(id) => page_resume(&store, id, args.take, args.unit, &mut out).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                (1, format!("unknown cursor '{id}' (state: {})", root.display()))
            } else {
                (1, format!("cannot resume '{id}': {e}"))
            }
        })?,
        None => {
            let mut input = Vec::new();
            io::stdin()
                .lock()
                .read_to_end(&mut input)
                .map_err(|e| (1, format!("cannot read stdin: {e}")))?;
            page_new(&store, &input, args.take, args.unit, &mut out)
                .map_err(|e| (1, format!("cannot page: {e}")))?
        }
    };
    out.flush().map_err(|e| (1, format!("cannot flush stdout: {e}")))?;

    // The trailer is the interface, and it lives on stderr by ruling.
    eprintln!("{}", render_with(schema, args.schema_template.as_deref(), &trailer));
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

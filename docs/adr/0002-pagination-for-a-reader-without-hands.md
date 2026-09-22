<!-- adr template version: "adr 3.11.5" -->

# ADR-0002: Pagination for a reader without hands — the design space

- **Date**: 2026-09-21
- **Iteration**: 1
- **Status**: Draft
- **Deciders**: Jérémie Lumbroso (rulings quoted below); the founding crew (recommendations to be answered)

**TL;DR**: `moreover` paginates arbitrary streams for readers that cannot
press a key: page out, trailer line with a cursor, resume by cursor in a
later invocation. Implementation is **Rust** (his ruling). Four design
questions are open below, each with a recommendation, so answering degrades
to accept/override.

---

## Originating Context

The one design question, as the naming deliberation put it: **how does a
reader without hands turn the page?** Interactive pagers hold the stream and
wait for a keypress; `moreover`'s reader gets its next chance to "press
space" in a *different process invocation*, possibly minutes later. So the
pause must become a **cursor**, and the held stream must survive between
invocations.

### DOC: the spec sketch (Jérémie, verbatim, 2026-09-21)

> ```console
> $ output | llmore -10
> [10 first lines]
> <llmore: page 1, 10/123 lines, cursor: Ae2e>
> $ llmore -c Ae2e --all
> [113 lines]
> <llmore: 123/123 lines, cursor: null>
> ```
>
> That's essentially pagination, with a cursor.

*(The tool has since been named `moreover` — ADR-0001 — so the trailer reads
`<moreover: …>`.)*

### DOC: the language ruling (Jérémie, verbatim, 2026-09-21)

> For io reasons, I think the tool should be in Rust.

### DOC: the trailer-line doctrine (from the naming strike — why the trailer is load-bearing)

> that trailer line is where the name does its daily work: it appears in
> every model's context at exactly the moment the model needs to know
> there's more.

## Questions

### QST-CURSOR-STORE: Where does the paged stream live between invocations?
- Status: answered
- Why asking: the pipe is consumed at first read; when the resume call comes, the original stream is gone. Whatever wasn't printed must have been spooled somewhere durable enough to outlive the first process.
- Need: a storage location + lifecycle (creation, lookup, expiry)

**Recommendation**: (by Operator 5, Claude Fable 5)
**A — spool files under `$XDG_STATE_HOME/moreover/` (fallback
`~/.local/state/moreover/`), one file per stream; the cursor id is a short
prefix of the spool's content hash.** *Rationale*: XDG *state* (not *cache*)
semantics match — caches may be evicted while a cursor is outstanding;
spool-per-stream keeps resumption O(seek); no daemon, trivially inspectable
with ordinary tools. *Confidence*: medium-high — the alternatives fail
structurally (re-reading the pipe is impossible; a daemon is heavy for a
tool this size). *If wrong*: if cursors ever need to work across machines or
users, a content-addressed spool with an explicit export subcommand replaces
the directory layout — the cursor grammar survives unchanged.

**ANS:** (by Jérémie Lumbroso)
Yes, you're calling it Option A, but I don't see any other options. I think both options are great. We should already think of where/how users are going to configure this tool (for instance to pick one or another spool files destination).

Other things that came to mind are tempfiles, but they are supposed to be garbage collectable after the process ends, whereas that's not what we want `moreover` to be able to do --- unless we lock them.

Here are more options, sourced from Google's AI mode:

```markdown
The best mechanism for passing data sequentially between processes on macOS or Unix using a spool file is a Named Pipe (also known as a FIFO), though you can also use traditional temporary files with file locking.
Here are the primary ways to achieve this, ranked by how well they handle process-to-process streaming:
## 1. Named Pipes (FIFOs) — The Best Streaming Option
A Named Pipe acts like a temporary file in the filesystem, but it behaves like a pipe (|). One process writes to it, and another reads from it sequentially. The data goes directly through memory without actually wearing out your physical storage drive.

* 
* How to create it: Use the command mkfifo /tmp/my_spool_pipe
* How it works:
* Process A writes data into the pipe: echo "data" > /tmp/my_spool_pipe (this blocks until a reader connects).
   * Process B reads data out of the pipe: cat /tmp/my_spool_pipe
* Pros: Highly efficient; no disk I/O; enforces sequential order.
* Cons: It is synchronous. The writer will pause and wait until the reader shows up to pull the data out.
* 

## 2. Standard Temp Files with flock — The Best Persistent Option
If you need Process A to write a huge dump of data completely, finish its job, and let Process B read it much later, a standard temporary file is the right choice. However, to prevent both processes from messing up the file at the same time, you must use file locking (flock).

* 
* The mechanism:
1. Process A creates a file in /tmp/spool.dat.
   2. Process A applies an exclusive lock using the flock system call (or the flock command-line utility) so no one else can touch it.
   3. Process A writes the data and unlocks it.
   4. Process B acquires a shared lock, safely reads the data, and then deletes the file.
* Pros: Asynchronous (Process A and B don't need to run at the same time); data persists if a process crashes.
* Cons: Hits the actual storage disk; requires careful management to delete the file when done.
* 

## 3. Memory-Mapped Files (mmap) — The Fastest Option
If both processes are running at the same time and need to share a massive spool of data instantly, they can map a temporary file directly into their shared RAM using the mmap() system call.

* 
* How it works: Process A creates a temporary file. Both processes call mmap() on that file descriptor. The operating system links their virtual memory to that file.
* Pros: Blazing fast. Writing to the "file" is just writing directly to RAM.
* Cons: Requires writing complex code (usually C, C++, or Python); requires synchronization tools like semaphores so they don't overwrite each other.
* 

------------------------------
## Comparison of Mechanisms

| Mechanism | Storage Type | Synchronization | Best Used For |
|---|---|---|---|
| Named Pipe (FIFO) | RAM (Virtual File) | Built-in (Blocks until read) | Live, streaming data between active apps. |
| Temp File + flock | Disk (/tmp/) | Manual (File locking) | Large data dumps where processes run at different times. |
| Shared Memory (mmap) | RAM | Manual (Semaphores/Mutexes) | High-performance apps needing instant data sharing. |
```

and more:

```markdown
In Rust, the ideal mechanism depends on whether your processes are running at the same time (streaming) or at completely different times (offline batching).
Because Rust handles concurrency safely, you can implement either approach cleanly using standard library features or popular crates.
------------------------------
## Method 1: Named Pipes (Streaming Data)
If both processes run concurrently, use a Named Pipe (FIFO). This uses the filesystem as a rendezvous point but streams data directly through kernel memory.
You can use the standard std::fs::OpenOptions to read and write to a pipe created by mkfifo.
## The Writer Process

use std::fs::OpenOptions;use std::io::Write;
fn main() -> std::io::Result<()> {
    // Note: The pipe must already exist (e.g., created via `mkfifo /tmp/my_pipe`)
    // Opening a FIFO for writing blocks until a reader opens it.
    let mut pipe = OpenOptions::new()
        .write(true)
        .open("/tmp/my_pipe")?;

    writeln!(pipe, "Spooling data stream from Rust process A!")?;
    Ok(())
}

## The Reader Process

use std::fs::File;use std::io::{BufRead, BufReader};
fn main() -> std::io::Result<()> {
    let file = File::open("/tmp/my_pipe")?;
    let mut reader = BufReader::new(file);

    let mut line = String::new();
    // Blocks and reads data as it arrives
    while reader.read_line(&mut line)? > 0 {
        print!("Received: {}", line);
        line.clear();
    }
    Ok(())
}

------------------------------
## Method 2: Temporary Files + Advisory Locking (Batch Spooling)
If Process A needs to dump a massive amount of data, close, and then let Process B read it later, use a real temporary file. To ensure Process B never reads a half-written file, use the fs2 or fd-lock crate for advisory file locking.
Add this to your Cargo.toml:

[dependencies]
tempfile = "3.10"  # Safely creates unique temp files
fs2 = "0.4"        # Cross-platform file locking

## The Writer Process

use fs2::FileExt; // Brings .lock_exclusive() into scopeuse std::fs::OpenOptions;use std::io::Write;
fn main() -> std::io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("/tmp/spool_file.dat")?;

    // Lock the file exclusively so the reader waits until we finish writing
    file.lock_exclusive()?;

    let mut writer = std::io::BufWriter::new(&file);
    writer.write_all(b"Bulk data spool dump here...")?;
    writer.flush()?;

    // Unlocking happens automatically when `file` goes out of scope,
    // or you can call file.unlock() explicitly.
    Ok(())
}

## The Reader Process

use fs2::FileExt; // Brings .lock_shared() into scopeuse std::fs::File;use std::io::Read;
fn main() -> std::io::Result<()> {
    let file = File::open("/tmp/spool_file.dat")?;

    // This will block until the writer releases its exclusive lock
    file.lock_shared()?;

    let mut content = String::new();
    let mut reader = std::io::BufReader::new(&file);
    reader.read_to_string(&mut content)?;

    println!("Read successfully: {}", content);
    
    // Clean up the spool file when done
    std::fs::remove_file("/tmp/spool_file.dat")?;
    Ok(())
}

------------------------------
## Which should you pick?

* Choose Named Pipes if you want low latency, zero disk wear, and your reader is active alongside your writer.
* Choose Temp Files + fs2 if your processes run asynchronously, or you need a persistent cache on disk in case one process crashes midway.
```

My hunch based on all of this is that: Temporary file + lock might be a good alternate option. For traceability, I think Option A is better, but the Temporary file + lock might be a good fallback or alternate option to already plan for.

---

### QST-CURSOR-SEMANTICS: What exactly does a cursor name?
- Status: answered
- Why asking: `moreover -c Ae2e --all` and a hypothetical `moreover -c Ae2e -20` must both make sense. Does the cursor carry only "which stream + where in it," with page size per-call, or does it also freeze paging parameters?
- Need: cursor = f(what)

**Recommendation**: (by Operator 5, Claude Fable 5)
**Stream identity + offset only; page size stays per-call.** *Rationale*:
the founding sketch shows exactly this shape — one cursor consumed by both
`--all` and a sized call; freezing parameters into the cursor would make the
sketch's second line impossible. *Confidence*: high — the sketch is the
spec. *If wrong*: optional fields can be added to the cursor's stored side
without changing its printed form; nothing breaks.

**ANS:** (by Jérémie Lumbroso)
Yes, a cursor maps to a (stream, offset) — it doesn't pre-apportion the page size. The cursor should be a base-32 identifier, as we've seen from ThirdX that these are appropriate as petnames for models. This is a good decoupling of position and output size.

---

### QST-PAGE-UNITS: Lines, bytes, or tokens?
- Status: answered
- Why asking: the reader is a model, and models budget in tokens — but tokenizers are model-specific, heavy dependencies, and the founding sketch speaks in lines.
- Need: v0 unit set + the upgrade path

**Recommendation**: (by Operator 5, Claude Fable 5)
**Lines by default, bytes via flag, in v0; tokens deferred to an opt-in
`--tokens` with a pluggable tokenizer.** *Rationale*: lines match the sketch
and fifty years of pager intuition; bytes are free to support; a bundled
tokenizer would tie a tiny tool to one model family's arithmetic.
*Confidence*: medium. *If wrong*: if real transcripts show models repeatedly
burned by mid-structure truncation that token-aware pages would have
avoided, promote tokens into v1 and say so in this ADR's iterations.

**ANS:** (by Jérémie Lumbroso)
Great recommendation, and I agree. Let's ensure to architect this in a way that is decoupled and in which we can switch different chunking mechanisms. We should also research what it would take to support a pluggable tokenizer — lots of work, or not, and if we can build on existing libraries. I think building a command line tool that natively understand the unit of the user is something that is really a ThirdX principle, the `-h` or `--human` flag that converts units into something that is intelligible for humans is an excellent design affordance to generalize and universalize.

---

### QST-TRAILER-GRAMMAR: Is the trailer line's grammar frozen as sketched?
- Status: answered
- Why asking: the trailer is the tool's most-seen surface (it lands in a model's context every page) and the founding record already published its shape; changing it later breaks the readers who learned it.
- Need: freeze/amend decision for v0, incl. the unknown-total case

**Recommendation**: (by Operator 5, Claude Fable 5)
**Freeze the sketch verbatim as v0** — `<moreover: page N, X/Y lines,
cursor: C>` — **with `Y = ?` when the total is unknown** (live streams) and
`cursor: null` at exhaustion. *Rationale*: it is already the published
founding grammar; models parse stable grammars; the one case the sketch
leaves open (unfinished input) needs exactly one symbol. *Confidence*: high
for v0. *If wrong*: introduce a versioned prefix (`<moreover/2: …>`) only on
a breaking need — never silently.

**ANS:** (by Jérémie Lumbroso)
It was my suggested grammar, but it was not a careful choice. We can definitely freeze this grammar, or tweak it. In general, I think we should anticipate the possibility of multiple grammars and preemptively address those challenges, for instance, including in the following way:
- design the logic for outputing this footer modularly
- make the command line tool able to provide the template it will use
- make the command line tool able to *change* the template it will use
- I think this footer should go to stderr?

So: We can version these templates, use the latest one, but also have that flag for the caller to request a specific "schema" of the footer, and also to ask what the "schema" looks like. Jinja2 type syntax, or whatever is best in Rust.

In terms of specific schemas, it's fine if we use exactly the one I gave for v0. Maybe at some later point we can A/B/...-test variations and see which models prefer.

---

## Consequences

- The cursor store (QST-CURSOR-STORE) is the only stateful component; every
  other choice is grammar. Settle it first and the implementation is mostly
  `io::copy` with a bookmark — which is why Rust's answer to "io reasons" is
  the whole runtime story.
- The trailer grammar, once frozen, is a compatibility promise to
  every model that reads it.

## Action Items

- [ ] Answer the four QSTs (accept or override the recommendations) - Owner: Jérémie + founding crew
- [ ] First implementation after (or judgment-first with defaults, recording deviations here) - Owner: founding crew
- [ ] `cargo build` green + first regression test per the Third Directive - Owner: founding crew

## Iterations

### Iteration 1 (2026-09-21)
- Trigger: founding. Spec sketch + Rust ruling staked verbatim; four questions opened with recommendations.
- Contributors: Jérémie Lumbroso (spec, ruling); Operator 5 (record, recommendations).
- Outcome: `— → Draft`

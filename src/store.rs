// The cursor store (ADR-0002 QST-CURSOR-STORE): the only stateful
// component. v0 backend: spool files + cursor records under
// $MOREOVER_STATE_DIR, else $XDG_STATE_HOME/moreover, else
// ~/.local/state/moreover — XDG *state*, never /tmp: nothing here may be
// garbage-collectable while a cursor is outstanding. The trait exists so
// the temp-file+flock alternate (or any other backend) can slot in
// without touching paging logic.

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// A cursor names a (stream, offset) — never a page size (QST-CURSOR-SEMANTICS).
/// `line` and `page` ride along so trailers can report position without
/// re-scanning the spool. `desk` is the mint-time working directory
/// (ADR-0003's desk scoping): shells die between a reader's invocations,
/// but the working directory survives — so `-c last` can mean "MY last"
/// on a machine full of concurrent readers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub spool: String,
    pub offset: u64,
    pub line: u64,
    pub page: u64,
    pub desk: String,
    /// Complete newlines before `offset` — the field that lets a bounded
    /// resume report exact line counts without rescanning the prefix
    /// (ADR-0006). `None` on records minted before it existed: those
    /// resume via the legacy whole-read path.
    pub nl: Option<u64>,
}

/// Spool metadata, written once at drain time (ADR-0006): totals the
/// trailer needs, so resume never rescans the spool to count them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpoolMeta {
    pub bytes: u64,
    /// Complete newlines in the spool.
    pub nl: u64,
    /// Whether the spool's last byte is a newline (decides if a trailing
    /// partial line counts toward the line total).
    pub ends_nl: bool,
}

impl SpoolMeta {
    /// Total lines under the house counting rule (a final unterminated
    /// line counts: a reader was shown it, the trailer must not lie).
    pub fn total_lines(&self) -> u64 {
        self.nl + u64::from(self.bytes > 0 && !self.ends_nl)
    }
}

/// Cursor-minting policy (ADR-0005): `Stable` derives the id from
/// (spool, offset, page) — the same resume repeated yields the same
/// next-cursor forever, and a million identical calls cost one record.
/// `Fresh` mints a random id per resume. Selected by assertive flags
/// ("flags assert destinations, never deltas"); stable is the built-in
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MintMode {
    Stable,
    Fresh,
}

pub trait Store {
    /// Persist a fully drained stream; returns the spool's name.
    fn put_spool(&self, data: &[u8]) -> io::Result<String>;
    fn read_spool(&self, name: &str) -> io::Result<Vec<u8>>;
    /// Drain-time totals, if this spool has them (`None` = legacy spool:
    /// resume falls back to the whole-read path).
    fn spool_meta(&self, name: &str) -> io::Result<Option<SpoolMeta>>;
    /// Seek + bounded read: exactly `[start, start+len)` clamped to the
    /// spool's end — the primitive that makes resume cost proportional
    /// to what was requested.
    fn read_spool_range(&self, name: &str, start: u64, len: u64) -> io::Result<Vec<u8>>;
    /// Persist a position; returns its base-32 id (derived or random,
    /// per `mode`). Cursors are immutable — resuming never rewrites one,
    /// it mints the next position's id, so re-reading a cursor is
    /// idempotent; under `Stable`, minting itself is idempotent too.
    fn put_cursor(&self, cursor: &Cursor, mode: MintMode) -> io::Result<String>;
    fn get_cursor(&self, id: &str) -> io::Result<Cursor>;
}

pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn open(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join("spools"))?;
        fs::create_dir_all(root.join("cursors"))?;
        Ok(FsStore { root })
    }

    /// Resolution order is the config surface ruled in ADR-0002:
    /// flag (caller passes it in) > env > XDG state > home fallback.
    pub fn default_root() -> PathBuf {
        if let Ok(dir) = std::env::var("MOREOVER_STATE_DIR") {
            if !dir.is_empty() {
                return PathBuf::from(dir);
            }
        }
        if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("moreover");
            }
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local/state/moreover")
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }
}

impl Store for FsStore {
    fn put_spool(&self, data: &[u8]) -> io::Result<String> {
        let name = format!("{:016x}", fnv1a64(data));
        let path = self.root.join("spools").join(&name);
        let meta_path = self.root.join("spools").join(format!("{name}.meta"));
        if !path.exists() {
            let tmp = path.with_extension("part");
            let mut f = fs::File::create(&tmp)?;
            f.lock()?; // exclusive while writing; released on close
            f.write_all(data)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&tmp, &path)?;
        }
        if !meta_path.exists() {
            // Drain-time totals (ADR-0006): counted once, here, so no
            // resume ever rescans the spool for them. Also heals legacy
            // spools the next time their content is re-piped.
            let nl = data.iter().filter(|&&b| b == b'\n').count() as u64;
            let ends_nl = data.last() == Some(&b'\n');
            let tmp = meta_path.with_extension("meta.part");
            let mut f = fs::File::create(&tmp)?;
            write!(f, "bytes={}\nnl={}\nends_nl={}\n", data.len(), nl, u8::from(ends_nl))?;
            f.sync_all()?;
            drop(f);
            fs::rename(&tmp, &meta_path)?;
        }
        Ok(name)
    }

    fn read_spool(&self, name: &str) -> io::Result<Vec<u8>> {
        let f = fs::File::open(self.root.join("spools").join(name))?;
        f.lock_shared()?;
        let mut data = Vec::new();
        (&f).read_to_end(&mut data)?;
        Ok(data)
    }

    fn spool_meta(&self, name: &str) -> io::Result<Option<SpoolMeta>> {
        let p = self.root.join("spools").join(format!("{name}.meta"));
        let text = match fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut bytes = None;
        let mut nl = None;
        let mut ends_nl = None;
        for l in text.lines() {
            match l.split_once('=') {
                Some(("bytes", v)) => bytes = v.parse().ok(),
                Some(("nl", v)) => nl = v.parse().ok(),
                Some(("ends_nl", v)) => ends_nl = v.parse::<u8>().ok().map(|x| x != 0),
                _ => {}
            }
        }
        // A corrupt sidecar is treated as absent: the legacy path is
        // always correct, just costlier.
        Ok(match (bytes, nl, ends_nl) {
            (Some(bytes), Some(nl), Some(ends_nl)) => Some(SpoolMeta { bytes, nl, ends_nl }),
            _ => None,
        })
    }

    fn read_spool_range(&self, name: &str, start: u64, len: u64) -> io::Result<Vec<u8>> {
        use std::io::{Seek, SeekFrom};
        let mut f = fs::File::open(self.root.join("spools").join(name))?;
        f.lock_shared()?;
        f.seek(SeekFrom::Start(start))?;
        let mut buf = vec![0u8; len as usize];
        let mut filled = 0;
        while filled < buf.len() {
            let n = f.read(&mut buf[filled..])?;
            if n == 0 {
                break; // clamped at the spool's end
            }
            filled += n;
        }
        buf.truncate(filled);
        Ok(buf)
    }

    fn put_cursor(&self, cursor: &Cursor, mode: MintMode) -> io::Result<String> {
        // Publication is ATOMIC (pre-release audit, finding 2): the
        // record is fully written and synced to a temp file, then
        // hard-linked to its final id — a contender never observes a
        // named-but-empty record, so "one record per triple" and "same
        // next-cursor" hold across the old publication window. Stable
        // mode derives the id from the identity triple with a
        // deterministic hash; an existing record is REUSED iff its
        // triple matches; a mismatched or unreadable occupant (legacy
        // debris) climbs the ladder. The ladder is BOUNDED to the id
        // space readers accept (finding 3): lengths 4..=16, eight rungs
        // each, then an explicit exhaustion error — every successful
        // mint round-trips through get_cursor. Desk recency is tracked
        // per-desk in `desks/` (finding 1), never by touching the shared
        // record, so one desk's reuse cannot disturb another's recovery.
        let mut record = format!(
            "spool={}\noffset={}\nline={}\npage={}\ndesk={}\n",
            cursor.spool, cursor.offset, cursor.line, cursor.page, cursor.desk
        );
        if let Some(nl) = cursor.nl {
            record.push_str(&format!("nl={nl}\n"));
        }
        record.push_str(if mode == MintMode::Stable { "mode=stable\n" } else { "mode=fresh\n" });

        // Exclusive temp creation (re-check finding 3): File::create
        // would TRUNCATE an existing name — and a temp that was already
        // hard-linked to a published cursor shares its inode, so a name
        // collision would rewrite a published record through the alias.
        // create_new establishes ownership; an occupied name just gets a
        // new suffix. Cleanup is guaranteed on every exit path below.
        let mut tmp;
        let mut f;
        loop {
            tmp = self
                .root
                .join("cursors")
                .join(format!(".mint-{}-{}", std::process::id(), mint_id(8)));
            match fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
                Ok(file) => {
                    f = file;
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        let staged = (|| -> io::Result<()> {
            f.write_all(record.as_bytes())?;
            f.sync_all()?;
            Ok(())
        })();
        if let Err(e) = staged {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        drop(f);
        let publish = |id: &str| -> io::Result<bool> {
            match fs::hard_link(&tmp, self.root.join("cursors").join(id)) {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
                Err(e) => Err(e),
            }
        };

        let mut rung: u64 = 0;
        let result = 'mint: {
            for len in 4..=16usize {
                for _ in 0..8 {
                    let id = match mode {
                        MintMode::Fresh => mint_id(len),
                        MintMode::Stable => derive_id(cursor, len, rung),
                    };
                    if publish(&id)? {
                        break 'mint Ok(id);
                    }
                    if mode == MintMode::Stable {
                        // occupant is complete by construction when ours;
                        // legacy debris may be unreadable → ladder.
                        if let Ok(existing) = self.get_cursor(&id) {
                            if existing.spool == cursor.spool
                                && existing.offset == cursor.offset
                                && existing.page == cursor.page
                            {
                                break 'mint Ok(id);
                            }
                        }
                        rung += 1;
                    }
                }
            }
            Err(io::Error::other(
                "cursor id space exhausted for this position — the state \
                 dir holds too many colliding records; clear old cursors \
                 and retry",
            ))
        };
        let _ = fs::remove_file(&tmp);
        let id = result?;

        // per-desk recency (finding 1; hardened per the re-check): the
        // caller's desk records its own last use; the shared record is
        // never modified. Each update stages in an EXCLUSIVELY OWNED
        // temp (re-check finding 2 — a shared .part inode let writers
        // truncate each other mid-publish) and any failure PROPAGATES:
        // a silent best-effort write cannot support the recovery
        // promise, so the error names the minted id the reader would
        // otherwise lose (re-check finding 1).
        if !cursor.desk.is_empty() {
            self.record_desk_use(&cursor.desk, &id).map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "cursor {id} was minted, but this directory's recovery \
                         record could not be updated ({e}) — resume with the \
                         printed id; `-c last` here may be stale until the \
                         state dir is writable"
                    ),
                )
            })?;
        }
        Ok(id)
    }

    fn get_cursor(&self, id: &str) -> io::Result<Cursor> {
        let id = normalize_id(id).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid cursor id: {id}"))
        })?;
        let text = fs::read_to_string(self.root.join("cursors").join(&id))?;
        let mut spool = None;
        let mut offset = None;
        let mut line = None;
        let mut page = None;
        let mut desk = String::new(); // absent in pre-desk records: tolerated
        let mut nl = None; // absent in pre-seek records: legacy resume path
        for l in text.lines() {
            match l.split_once('=') {
                Some(("spool", v)) => spool = Some(v.to_string()),
                Some(("offset", v)) => offset = v.parse().ok(),
                Some(("line", v)) => line = v.parse().ok(),
                Some(("page", v)) => page = v.parse().ok(),
                Some(("desk", v)) => desk = v.to_string(),
                Some(("nl", v)) => nl = v.parse().ok(),
                _ => {}
            }
        }
        match (spool, offset, line, page) {
            (Some(spool), Some(offset), Some(line), Some(page)) => {
                Ok(Cursor { spool, offset, line, page, desk, nl })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("corrupt cursor record: {id}"),
            )),
        }
    }
}

impl FsStore {
    /// Write this desk's recovery record: staged in an exclusively owned
    /// temp, published whole by atomic rename (last-writer-wins is the
    /// correct semantics for recency; sharing a staging inode was not).
    fn record_desk_use(&self, desk: &str, id: &str) -> io::Result<()> {
        fs::create_dir_all(self.root.join("desks"))?;
        let dpath = self.root.join("desks").join(format!("{:016x}", fnv1a64(desk.as_bytes())));
        let mut dtmp;
        let mut df;
        loop {
            dtmp = self
                .root
                .join("desks")
                .join(format!(".part-{}-{}", std::process::id(), mint_id(8)));
            match fs::OpenOptions::new().write(true).create_new(true).open(&dtmp) {
                Ok(file) => {
                    df = file;
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        let staged = (|| -> io::Result<()> {
            write!(df, "desk={desk}\nid={id}\n")?;
            df.sync_all()?;
            Ok(())
        })();
        drop(df);
        if let Err(e) = staged {
            let _ = fs::remove_file(&dtmp);
            return Err(e);
        }
        if let Err(e) = fs::rename(&dtmp, &dpath) {
            let _ = fs::remove_file(&dtmp);
            return Err(e);
        }
        Ok(())
    }

    /// The newest cursor MINTED OR REUSED from `desk` — `-c last`'s
    /// resolver (ADR-0003; shipped on first-contact field evidence).
    /// Primary source: the per-desk recency file (audit finding 1 —
    /// under stable minting a record's stored desk is its FIRST
    /// writer's, so desk recovery must never route through the shared
    /// record). Fallback for pre-upgrade state: the record scan.
    pub fn last_cursor_for_desk(&self, desk: &str) -> io::Result<Option<String>> {
        if desk.is_empty() {
            return Ok(None);
        }
        let dpath = self.root.join("desks").join(format!("{:016x}", fnv1a64(desk.as_bytes())));
        match fs::read_to_string(&dpath) {
            Ok(text) => {
                let mut d = None;
                let mut id = None;
                for l in text.lines() {
                    match l.split_once('=') {
                        Some(("desk", v)) => d = Some(v.to_string()),
                        Some(("id", v)) => id = Some(v.to_string()),
                        _ => {}
                    }
                }
                match (d, id) {
                    (Some(d), Some(id)) if d == desk => {
                        return match self.get_cursor(&id) {
                            Ok(_) => Ok(Some(id)),
                            // a recorded-but-missing cursor is damage, not
                            // legacy absence: report it rather than silently
                            // selecting an older stream (re-check finding 1)
                            Err(e) => Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "this directory's recovery record points at \
                                     cursor {id}, which cannot be read ({e}) — \
                                     resume with a printed id, or clear {} to reset",
                                    dpath.display()
                                ),
                            )),
                        };
                    }
                    (Some(d), _) if d != desk => {} // hash collision: fall through to scan
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "this directory's recovery record is malformed — \
                                 resume with a printed id, or clear {} to reset",
                                dpath.display()
                            ),
                        ));
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {} // legacy absence: scan
            Err(e) => return Err(e),
        }
        // legacy fallback: scan records by their stored (first-writer) desk
        let mut best: Option<(std::time::SystemTime, String)> = None;
        for entry in fs::read_dir(self.root.join("cursors"))? {
            let entry = entry?;
            let id = match entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let cursor = match self.get_cursor(&id) {
                Ok(c) => c,
                Err(_) => continue, // a corrupt record shouldn't break `last`
            };
            if cursor.desk != desk {
                continue;
            }
            let mtime = entry.metadata()?.modified()?;
            if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                best = Some((mtime, id));
            }
        }
        Ok(best.map(|(_, id)| id))
    }
}

/// Crockford base-32 (QST-CURSOR-SEMANTICS: base-32 petnames, read
/// case-insensitively). Generation uses the lowercase alphabet; reading
/// folds case and the Crockford confusables (o→0, i/l→1).
const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Stable-mode id derivation (ADR-0005): a keyed hash over the identity
/// triple, rendered in the same alphabet with the same mixed
/// letter+digit guarantee as random minting — candidates that come out
/// all-digit or all-letter are re-hashed with a counter, deterministically,
/// so replaying a mint always lands on the same id.
fn derive_id(cursor: &Cursor, len: usize, rung: u64) -> String {
    let mut salt: u64 = 0;
    loop {
        let key = format!("{}\x1f{}\x1f{}\x1f{}\x1f{}", cursor.spool, cursor.offset, cursor.page, rung, salt);
        let mut h = fnv1a64(key.as_bytes());
        let id: String = (0..len)
            .map(|_| {
                let c = ALPHABET[(h % 32) as usize] as char;
                h /= 32;
                // re-mix so ids longer than 12 chars don't run dry
                if h < 32 {
                    h = fnv1a64(&h.to_le_bytes());
                }
                c
            })
            .collect();
        let has_digit = id.bytes().any(|b| b.is_ascii_digit());
        let has_letter = id.bytes().any(|b| b.is_ascii_alphabetic());
        if has_digit && has_letter {
            return id;
        }
        salt += 1;
    }
}

fn mint_id(len: usize) -> String {
    // Ids must mix at least one letter and one digit: an all-digit id
    // reads as a count and invites the reader to extrapolate a sequence;
    // an all-letter id reads as a word. Mixed, code-like ids are the form
    // model readers reliably retype (estate petname doctrine, per ThirdX
    // practice; internal experiments informed this).
    loop {
        let mut bytes = vec![0u8; len];
        fill_random(&mut bytes);
        let id: String = bytes.iter().map(|b| ALPHABET[(*b as usize) % 32] as char).collect();
        let has_digit = id.bytes().any(|b| b.is_ascii_digit());
        let has_letter = id.bytes().any(|b| b.is_ascii_alphabetic());
        if has_digit && has_letter {
            return id;
        }
    }
}

fn fill_random(buf: &mut [u8]) {
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        if f.read_exact(buf).is_ok() {
            return;
        }
    }
    // Fallback (no /dev/urandom): time-derived — uniqueness, not secrecy;
    // cursor ids are petnames, never credentials.
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64).rotate_left(32);
    for b in buf.iter_mut() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = (seed >> 33) as u8;
    }
}

pub fn normalize_id(id: &str) -> Option<String> {
    if id.is_empty() || id.len() > 16 {
        return None;
    }
    let mut out = String::with_capacity(id.len());
    for c in id.chars() {
        let c = match c.to_ascii_lowercase() {
            'o' => '0',
            'i' | 'l' => '1',
            c if ALPHABET.contains(&(c as u8)) => c,
            _ => return None,
        };
        out.push(c);
    }
    Some(out)
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ladder_is_bounded_and_exhaustion_is_explicit() {
        // Pre-release audit, finding 3: the unbounded ladder could mint
        // a 17-char id that normalize_id then rejects — a record its own
        // reader refuses. The ladder is now capped to the accepted id
        // space (4..=16), and running it dry is an explicit error, never
        // an unusable record. This occupies every candidate the way the
        // audit's probe did: valid records for a DIFFERENT triple.
        let dir = std::env::temp_dir().join(format!("moreover-ladder-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = FsStore::open(dir.clone()).unwrap();
        let target = Cursor {
            spool: "0123456789abcdef".into(),
            offset: 42,
            line: 0,
            page: 2,
            desk: String::new(),
            nl: Some(0),
        };
        let decoy = "spool=ffffffffffffffff\noffset=9\nline=0\npage=9\ndesk=\nnl=0\nmode=stable\n";
        let mut occupied = 0;
        let mut rung = 0u64;
        for len in 4..=16usize {
            for _ in 0..8 {
                let id = derive_id(&target, len, rung);
                let p = dir.join("cursors").join(&id);
                if !p.exists() {
                    fs::write(&p, decoy).unwrap();
                }
                occupied += 1;
                rung += 1;
            }
        }
        assert_eq!(occupied, 104);
        let err = store.put_cursor(&target, MintMode::Stable).unwrap_err();
        assert!(err.to_string().contains("exhausted"), "got: {err}");
        // and no unusable record was left behind for the target triple
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cursor_ids_read_case_insensitively_with_crockford_folds() {
        // A model retyping "Ae2e" as "AE2E" (or OCR turning 0 into O)
        // must land on the same cursor — that is the base-32 ruling's
        // whole point.
        assert_eq!(normalize_id("AE2E").as_deref(), Some("ae2e"));
        assert_eq!(normalize_id("aO1l").as_deref(), Some("a011"));
        assert_eq!(normalize_id("has space"), None);
        assert_eq!(normalize_id(""), None);
    }
}

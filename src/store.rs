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
/// re-scanning the spool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub spool: String,
    pub offset: u64,
    pub line: u64,
    pub page: u64,
}

pub trait Store {
    /// Persist a fully drained stream; returns the spool's name.
    fn put_spool(&self, data: &[u8]) -> io::Result<String>;
    fn read_spool(&self, name: &str) -> io::Result<Vec<u8>>;
    /// Persist a position; returns its freshly minted base-32 id.
    /// Cursors are immutable — resuming never rewrites one, it mints the
    /// next position's id, so re-reading a cursor is idempotent.
    fn put_cursor(&self, cursor: &Cursor) -> io::Result<String>;
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
        if path.exists() {
            // Content-addressed: same bytes, same spool — rerunning a
            // pipeline never duplicates state.
            return Ok(name);
        }
        let tmp = path.with_extension("part");
        let mut f = fs::File::create(&tmp)?;
        f.lock()?; // exclusive while writing; released on close
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, &path)?;
        Ok(name)
    }

    fn read_spool(&self, name: &str) -> io::Result<Vec<u8>> {
        let f = fs::File::open(self.root.join("spools").join(name))?;
        f.lock_shared()?;
        let mut data = Vec::new();
        (&f).read_to_end(&mut data)?;
        Ok(data)
    }

    fn put_cursor(&self, cursor: &Cursor) -> io::Result<String> {
        // Short ids collide eventually; create_new makes the mint atomic
        // and we grow the id one character per retry round.
        let mut len = 4;
        loop {
            for _ in 0..8 {
                let id = mint_id(len);
                let path = self.root.join("cursors").join(&id);
                match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                    Ok(mut f) => {
                        write!(
                            f,
                            "spool={}\noffset={}\nline={}\npage={}\n",
                            cursor.spool, cursor.offset, cursor.line, cursor.page
                        )?;
                        f.sync_all()?;
                        return Ok(id);
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(e),
                }
            }
            len += 1;
        }
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
        for l in text.lines() {
            match l.split_once('=') {
                Some(("spool", v)) => spool = Some(v.to_string()),
                Some(("offset", v)) => offset = v.parse().ok(),
                Some(("line", v)) => line = v.parse().ok(),
                Some(("page", v)) => page = v.parse().ok(),
                _ => {}
            }
        }
        match (spool, offset, line, page) {
            (Some(spool), Some(offset), Some(line), Some(page)) => {
                Ok(Cursor { spool, offset, line, page })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("corrupt cursor record: {id}"),
            )),
        }
    }
}

/// Crockford base-32 (QST-CURSOR-SEMANTICS: base-32 petnames, read
/// case-insensitively). Generation uses the lowercase alphabet; reading
/// folds case and the Crockford confusables (o→0, i/l→1).
const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

fn mint_id(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    fill_random(&mut bytes);
    bytes.iter().map(|b| ALPHABET[(*b as usize) % 32] as char).collect()
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

// Page units (ADR-0002 QST-PAGE-UNITS): this boundary decouples "what is
// a unit" from paging. v0 ships lines and bytes; tokens arrive later
// behind the same three functions, never inside the paging logic.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Lines,
    Bytes,
}

impl Unit {
    /// The word the trailer prints — part of the frozen v0 grammar.
    pub fn label(self) -> &'static str {
        match self {
            Unit::Lines => "lines",
            Unit::Bytes => "bytes",
        }
    }
}

/// Total units in `data`. A final line without a trailing newline still
/// counts as a line (a reader was shown it; the trailer must not lie).
pub fn total_units(data: &[u8], unit: Unit) -> usize {
    match unit {
        Unit::Bytes => data.len(),
        Unit::Lines => count_lines(data),
    }
}

fn count_lines(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    let newlines = data.iter().filter(|&&b| b == b'\n').count();
    if data[data.len() - 1] == b'\n' {
        newlines
    } else {
        newlines + 1
    }
}

/// Byte offset after advancing `count` units from `from`, clamped to the
/// end of `data`. Lines advance past their newline, so a page never ends
/// mid-line in line mode.
pub fn advance(data: &[u8], from: usize, count: usize, unit: Unit) -> usize {
    match unit {
        Unit::Bytes => from.saturating_add(count).min(data.len()),
        Unit::Lines => {
            let mut pos = from;
            let mut seen = 0;
            while pos < data.len() && seen < count {
                match data[pos..].iter().position(|&b| b == b'\n') {
                    Some(i) => pos += i + 1,
                    None => pos = data.len(),
                }
                seen += 1;
            }
            pos
        }
    }
}

/// Byte offset after retreating `count` units back from `from`, clamped
/// to the start. In line mode it lands on a line start, so an overlap
/// reprint never begins mid-line.
pub fn retreat(data: &[u8], from: usize, count: usize, unit: Unit) -> usize {
    let from = from.min(data.len());
    match unit {
        Unit::Bytes => from.saturating_sub(count),
        Unit::Lines => {
            if from == 0 || count == 0 {
                return from;
            }
            let mut starts = vec![0usize];
            for i in 0..from {
                if data[i] == b'\n' && i + 1 < from {
                    starts.push(i + 1);
                }
            }
            starts[starts.len().saturating_sub(count)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retreat_lands_on_line_starts() {
        let data = b"one\ntwo\nthree\nfour\n";
        // from the start of "four" (14), back 2 lines => start of "two" (4)
        assert_eq!(retreat(data, 14, 2, Unit::Lines), 4);
        assert_eq!(retreat(data, 14, 99, Unit::Lines), 0);
        assert_eq!(retreat(data, 0, 3, Unit::Lines), 0);
        assert_eq!(retreat(data, 10, 4, Unit::Bytes), 6);
    }

    #[test]
    fn final_line_without_newline_counts() {
        // Real pipelines end without trailing newlines all the time
        // (printf, truncated logs); the totals must match what a reader
        // actually sees.
        assert_eq!(total_units(b"a\nb\nc", Unit::Lines), 3);
        assert_eq!(total_units(b"a\nb\nc\n", Unit::Lines), 3);
        assert_eq!(total_units(b"", Unit::Lines), 0);
    }

    #[test]
    fn line_advance_never_splits_a_line() {
        let data = b"one\ntwo\nthree\n";
        assert_eq!(advance(data, 0, 1, Unit::Lines), 4); // past "one\n"
        assert_eq!(advance(data, 4, 1, Unit::Lines), 8); // past "two\n"
        assert_eq!(advance(data, 0, 99, Unit::Lines), data.len());
    }
}

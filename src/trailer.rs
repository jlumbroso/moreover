// The trailer line (ADR-0002 QST-TRAILER-GRAMMAR): the tool's most-seen
// surface and a compatibility promise to every model that reads it.
// Rendering is modular by ruling: named schemas, inspectable
// (--schema-show) and replaceable (--schema-template). v0 templating is
// bare {placeholder} substitution ON PURPOSE — if schemas ever need
// logic, minijinja is the recorded upgrade path (ADR-0002 Decision §4).
//
// The v0 grammar is frozen verbatim from the founding sketch, both
// observed forms. Changing it is an ADR event, never a diff.

use crate::chunker::Unit;

#[derive(Debug, Clone)]
pub struct Schema {
    pub name: &'static str,
    /// Sized-page form: `<moreover: page 1, 10/123 lines, cursor: Ae2e>`
    pub paged: &'static str,
    /// --all / exhaustion form: `<moreover: 123/123 lines, cursor: null>`
    pub all: &'static str,
}

pub const V0: Schema = Schema {
    name: "v0",
    paged: "<moreover: page {page}, {shown}/{total} {unit}, cursor: {cursor}>",
    all: "<moreover: {shown}/{total} {unit}, cursor: {cursor}>",
};

pub fn lookup(name: &str) -> Option<&'static Schema> {
    match name {
        "v0" => Some(&V0),
        _ => None,
    }
}

/// Everything a trailer can say about a delivery.
#[derive(Debug, Clone)]
pub struct TrailerData {
    pub page: u64,
    /// Cumulative units delivered through the end of this page.
    pub shown: usize,
    pub total: usize,
    pub unit: Unit,
    /// None = exhausted → prints the literal `null`.
    pub cursor: Option<String>,
    /// Whether this delivery was --all (selects the schema's `all` form).
    pub all: bool,
}

pub fn render(template: &str, d: &TrailerData) -> String {
    let cursor = d.cursor.clone().unwrap_or_else(|| "null".to_string());
    template
        .replace("{page}", &d.page.to_string())
        .replace("{shown}", &d.shown.to_string())
        .replace("{total}", &d.total.to_string())
        .replace("{unit}", d.unit.label())
        .replace("{cursor}", &cursor)
}

/// Render with a schema, honoring an optional caller-supplied override
/// template ("able to change the template" — ADR-0002).
pub fn render_with(schema: &Schema, override_template: Option<&str>, d: &TrailerData) -> String {
    match override_template {
        Some(t) => render(t, d),
        None => render(if d.all { schema.all } else { schema.paged }, d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_grammar_matches_the_founding_sketch_exactly() {
        // These two strings are the published founding grammar
        // (ADR-0002 DOC: the spec sketch, renamed per ADR-0001).
        // If this test fails, a compatibility promise broke.
        let first = TrailerData {
            page: 1,
            shown: 10,
            total: 123,
            unit: Unit::Lines,
            cursor: Some("ae2e".to_string()),
            all: false,
        };
        assert_eq!(
            render_with(&V0, None, &first),
            "<moreover: page 1, 10/123 lines, cursor: ae2e>"
        );
        let last = TrailerData {
            page: 2,
            shown: 123,
            total: 123,
            unit: Unit::Lines,
            cursor: None,
            all: true,
        };
        assert_eq!(render_with(&V0, None, &last), "<moreover: 123/123 lines, cursor: null>");
    }

    #[test]
    fn caller_template_overrides_the_schema() {
        let d = TrailerData {
            page: 3,
            shown: 30,
            total: 40,
            unit: Unit::Bytes,
            cursor: Some("k7".to_string()),
            all: false,
        };
        assert_eq!(
            render_with(&V0, Some("[{shown}/{total} {unit} @{cursor}]"), &d),
            "[30/40 bytes @k7]"
        );
    }
}

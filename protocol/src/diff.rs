//! A line-level diff between two buffer texts, the basis of the inline
//! diff/apply review. Pure logic with no dependencies, so it runs the same on
//! the page (rendering a proposed edit as red/green) and, later, in a shared
//! changeset review. The page stages a proposal as full replacement text; this
//! turns the old and proposed text into a line sequence the editor paints and
//! groups into hunks for per-hunk accept and reject.

/// What happened to one line: kept from both sides, added by the new text, or
/// removed from the old text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineChange {
    Equal,
    Insert,
    Delete,
}

/// One line of the diff, tagged with how it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub change: LineChange,
    pub text: String,
}

/// A contiguous run of inserts and deletes with no kept line between them, the
/// unit of accept and reject. `new_start` is the 0-based line in the new text
/// where the run's inserts begin (and where deletes sit, as a zero-height
/// marker, since deleted lines are not in the new text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub new_start: usize,
    pub lines: Vec<DiffLine>,
}

/// The most cells the line-level diff will fill before giving up on the longest
/// common subsequence and returning a whole-buffer replacement. Keeps the
/// `O(old * new)` table bounded for very large or very different buffers.
const MAX_CELLS: usize = 4_000_000;

/// The line diff of `old` into `new`: every line in order, tagged `Equal`,
/// `Insert`, or `Delete`. Lines split on `\n`, so a trailing newline shows as a
/// final empty line on both sides.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.split('\n').collect();
    let new_lines: Vec<&str> = new.split('\n').collect();
    let rows = old_lines.len();
    let columns = new_lines.len();

    if rows.saturating_mul(columns) > MAX_CELLS {
        return replace_all(&old_lines, &new_lines);
    }

    let mut table = vec![vec![0u32; columns + 1]; rows + 1];
    for row in (0..rows).rev() {
        for column in (0..columns).rev() {
            table[row][column] = if old_lines[row] == new_lines[column] {
                table[row + 1][column + 1] + 1
            } else {
                table[row + 1][column].max(table[row][column + 1])
            };
        }
    }

    let mut out = Vec::new();
    let (mut row, mut column) = (0, 0);
    while row < rows && column < columns {
        if old_lines[row] == new_lines[column] {
            out.push(line(LineChange::Equal, new_lines[column]));
            row += 1;
            column += 1;
        } else if table[row + 1][column] >= table[row][column + 1] {
            out.push(line(LineChange::Delete, old_lines[row]));
            row += 1;
        } else {
            out.push(line(LineChange::Insert, new_lines[column]));
            column += 1;
        }
    }
    while row < rows {
        out.push(line(LineChange::Delete, old_lines[row]));
        row += 1;
    }
    while column < columns {
        out.push(line(LineChange::Insert, new_lines[column]));
        column += 1;
    }
    out
}

/// The diff grouped into hunks: each run of changes between two kept lines,
/// with the new-text line where it starts. Equal lines are the boundaries and
/// are not carried in any hunk.
pub fn hunks(old: &str, new: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut new_line = 0usize;
    let mut current: Option<Hunk> = None;
    for line in diff_lines(old, new) {
        match line.change {
            LineChange::Equal => {
                if let Some(hunk) = current.take() {
                    hunks.push(hunk);
                }
                new_line += 1;
            }
            LineChange::Insert => {
                current
                    .get_or_insert_with(|| Hunk {
                        new_start: new_line,
                        lines: Vec::new(),
                    })
                    .lines
                    .push(line);
                new_line += 1;
            }
            LineChange::Delete => {
                current
                    .get_or_insert_with(|| Hunk {
                        new_start: new_line,
                        lines: Vec::new(),
                    })
                    .lines
                    .push(line);
            }
        }
    }
    if let Some(hunk) = current.take() {
        hunks.push(hunk);
    }
    hunks
}

fn line(change: LineChange, text: &str) -> DiffLine {
    DiffLine {
        change,
        text: text.to_string(),
    }
}

fn replace_all(old_lines: &[&str], new_lines: &[&str]) -> Vec<DiffLine> {
    let mut out = Vec::with_capacity(old_lines.len() + new_lines.len());
    out.extend(old_lines.iter().map(|text| line(LineChange::Delete, text)));
    out.extend(new_lines.iter().map(|text| line(LineChange::Insert, text)));
    out
}

#[cfg(test)]
mod tests {
    use super::{LineChange::*, *};

    fn tags(old: &str, new: &str) -> Vec<LineChange> {
        diff_lines(old, new)
            .into_iter()
            .map(|line| line.change)
            .collect()
    }

    #[test]
    fn identical_is_all_equal() {
        assert_eq!(tags("a\nb\nc", "a\nb\nc"), vec![Equal, Equal, Equal]);
    }

    #[test]
    fn pure_insert_in_the_middle() {
        assert_eq!(tags("a\nc", "a\nb\nc"), vec![Equal, Insert, Equal]);
    }

    #[test]
    fn pure_delete_in_the_middle() {
        assert_eq!(tags("a\nb\nc", "a\nc"), vec![Equal, Delete, Equal]);
    }

    #[test]
    fn a_changed_line_is_delete_then_insert() {
        assert_eq!(
            tags("a\nb\nc", "a\nB\nc"),
            vec![Equal, Delete, Insert, Equal]
        );
    }

    #[test]
    fn filling_an_empty_buffer_inserts_after_its_blank_line() {
        // An empty buffer is one empty line, so it diffs as the blank line
        // dropped and the new lines inserted.
        assert_eq!(tags("", "a\nb"), vec![Delete, Insert, Insert]);
    }

    #[test]
    fn carries_the_line_text() {
        let lines = diff_lines("keep\ndrop", "keep\nadd");
        assert_eq!(lines[0], line(Equal, "keep"));
        assert!(lines.iter().any(|l| l.change == Delete && l.text == "drop"));
        assert!(lines.iter().any(|l| l.change == Insert && l.text == "add"));
    }

    #[test]
    fn hunks_group_runs_and_track_the_new_line() {
        // old: a b c d   new: a X c d e  -> change at line 1, append at line 4
        let grouped = hunks("a\nb\nc\nd", "a\nX\nc\nd\ne");
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].new_start, 1);
        assert_eq!(grouped[0].lines, vec![line(Delete, "b"), line(Insert, "X")]);
        assert_eq!(grouped[1].new_start, 4);
        assert_eq!(grouped[1].lines, vec![line(Insert, "e")]);
    }

    #[test]
    fn no_changes_yield_no_hunks() {
        assert!(hunks("a\nb", "a\nb").is_empty());
    }
}

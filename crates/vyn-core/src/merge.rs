use similar::{ChangeTag, TextDiff};

pub enum MergeOutcome {
    Merged(String),
    Conflicted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryMergeDecision {
    NoConflict,
    KeepBoth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Kept(String),     // side kept the base line unchanged
    Changed(String),  // side replaced the base line with new content
    Deleted,          // side deleted the base line
    Inserted(String), // side inserted a line with no base counterpart
}

/// True LCS-based 3-way merge. Lines only one side changed are taken
/// automatically; lines both sides changed differently produce conflict markers.
pub fn merge_text(base: &str, local: &str, remote: &str) -> MergeOutcome {
    if local == remote {
        return MergeOutcome::Merged(local.to_string());
    }
    if local == base {
        return MergeOutcome::Merged(remote.to_string());
    }
    if remote == base {
        return MergeOutcome::Merged(local.to_string());
    }

    let local_ops = side_ops(base, local);
    let remote_ops = side_ops(base, remote);

    let n = local_ops.len().max(remote_ops.len());
    let mut merged: Vec<String> = Vec::new();
    let mut has_conflict = false;

    for i in 0..n {
        let l = local_ops.get(i);
        let r = remote_ops.get(i);

        match (l, r) {
            // Both kept the same base line (or both agree on the same result)
            (Some(Op::Kept(ls)), Some(Op::Kept(_))) => merged.push(ls.clone()),

            // Only local changed; remote kept base -- take local
            (Some(Op::Changed(ls)), Some(Op::Kept(_))) => merged.push(ls.clone()),
            (Some(Op::Inserted(ls)), None) => merged.push(ls.clone()),

            // Only remote changed; local kept base -- take remote
            (Some(Op::Kept(_)), Some(Op::Changed(rs))) => merged.push(rs.clone()),
            (None, Some(Op::Inserted(rs))) => merged.push(rs.clone()),

            // Both inserted identical content
            (Some(Op::Inserted(ls)), Some(Op::Inserted(rs))) if ls == rs => merged.push(ls.clone()),

            // Both changed to the same value -- clean
            (Some(Op::Changed(ls)), Some(Op::Changed(rs))) if ls == rs => merged.push(ls.clone()),

            // Both deleted
            (Some(Op::Deleted), Some(Op::Deleted)) => {}

            // Local deleted, remote kept base -- take delete
            (Some(Op::Deleted), Some(Op::Kept(_))) => {}

            // Remote deleted, local kept base -- take delete
            (Some(Op::Kept(_)), Some(Op::Deleted)) => {}

            // Everything else is a conflict
            (l, r) => {
                has_conflict = true;
                merged.push("<<<<<<< LOCAL".to_string());
                match l {
                    Some(Op::Kept(s) | Op::Changed(s) | Op::Inserted(s)) => merged.push(s.clone()),
                    Some(Op::Deleted) | None => {}
                }
                merged.push("=======".to_string());
                match r {
                    Some(Op::Kept(s) | Op::Changed(s) | Op::Inserted(s)) => merged.push(s.clone()),
                    Some(Op::Deleted) | None => {}
                }
                merged.push(">>>>>>> REMOTE".to_string());
            }
        }
    }

    let body = if merged.is_empty() {
        String::new()
    } else {
        format!("{}\n", merged.join("\n"))
    };

    if has_conflict {
        MergeOutcome::Conflicted(body)
    } else {
        MergeOutcome::Merged(body)
    }
}

/// For each line in `base`, compute what `side` does with it.
/// Pure insertions (no base counterpart) are appended at the end.
fn side_ops(base: &str, side: &str) -> Vec<Op> {
    let diff = TextDiff::from_lines(base, side);
    let changes: Vec<_> = diff.iter_all_changes().collect();
    let mut ops: Vec<Op> = Vec::new();

    let mut i = 0;
    while i < changes.len() {
        match changes[i].tag() {
            ChangeTag::Equal => {
                ops.push(Op::Kept(
                    changes[i].value().trim_end_matches('\n').to_string(),
                ));
                i += 1;
            }
            ChangeTag::Delete => {
                // A Delete immediately followed by an Insert = replacement
                if i + 1 < changes.len() && changes[i + 1].tag() == ChangeTag::Insert {
                    ops.push(Op::Changed(
                        changes[i + 1].value().trim_end_matches('\n').to_string(),
                    ));
                    i += 2;
                } else {
                    ops.push(Op::Deleted);
                    i += 1;
                }
            }
            ChangeTag::Insert => {
                // Pure insertion: no base line consumed
                ops.push(Op::Inserted(
                    changes[i].value().trim_end_matches('\n').to_string(),
                ));
                i += 1;
            }
        }
    }

    ops
}

pub fn detect_binary_conflict(base: &[u8], local: &[u8], remote: &[u8]) -> BinaryMergeDecision {
    if local == remote || local == base || remote == base {
        BinaryMergeDecision::NoConflict
    } else {
        BinaryMergeDecision::KeepBoth
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryMergeDecision, MergeOutcome, detect_binary_conflict, merge_text};

    #[test]
    fn merge_non_overlapping_edits() {
        let base = "A=1\nB=2\n";
        let local = "A=9\nB=2\n";
        let remote = "A=1\nB=8\n";

        match merge_text(base, local, remote) {
            MergeOutcome::Merged(merged) => {
                assert!(merged.contains("A=9"), "local change missing");
                assert!(merged.contains("B=8"), "remote change missing");
            }
            MergeOutcome::Conflicted(_) => panic!("expected auto merge for non-overlapping edits"),
        }
    }

    #[test]
    fn merge_overlapping_conflict() {
        let conflict_local = "A=9\n";
        let conflict_remote = "A=8\n";
        match merge_text("A=1\n", conflict_local, conflict_remote) {
            MergeOutcome::Conflicted(text) => {
                assert!(text.contains("<<<<<<< LOCAL"));
                assert!(text.contains(">>>>>>> REMOTE"));
            }
            MergeOutcome::Merged(_) => panic!("expected conflict markers for overlapping edits"),
        }
    }

    #[test]
    fn merge_insertion_by_one_side() {
        let base = "line1\nline2\n";
        let local = "line1\nline2\nline3\n";
        let remote = "line1\nline2\n";
        match merge_text(base, local, remote) {
            MergeOutcome::Merged(m) => assert!(m.contains("line3")),
            MergeOutcome::Conflicted(_) => panic!("expected clean merge"),
        }
    }

    #[test]
    fn binary_conflict_detection() {
        let base = b"abc";
        let local = b"abc";
        let remote = b"abd";
        assert_eq!(
            detect_binary_conflict(base, local, remote),
            BinaryMergeDecision::NoConflict
        );

        let local = b"xyz";
        let remote = b"123";
        assert_eq!(
            detect_binary_conflict(base, local, remote),
            BinaryMergeDecision::KeepBoth
        );
    }
}

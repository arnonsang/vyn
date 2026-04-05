use similar::{ChangeTag, TextDiff};

pub fn is_binary(data: &[u8]) -> bool {
    let probe = &data[..data.len().min(1024)];
    if probe.contains(&0) {
        return true;
    }

    std::str::from_utf8(probe).is_err()
}

pub fn unified_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();
    output.push_str(&format!("--- {old_label}\n"));
    output.push_str(&format!("+++ {new_label}\n"));

    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            match change.tag() {
                ChangeTag::Delete => output.push('-'),
                ChangeTag::Insert => output.push('+'),
                ChangeTag::Equal => output.push(' '),
            }
            output.push_str(&change.to_string());
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{is_binary, unified_diff};

    #[test]
    fn line_diff_logic() {
        let old = "A=1\nB=2\n";
        let new = "A=1\nB=3\nC=4\n";
        let rendered = unified_diff(old, new, "old/.env", "new/.env");

        assert!(rendered.contains("--- old/.env"));
        assert!(rendered.contains("+++ new/.env"));
        assert!(rendered.contains("-B=2"));
        assert!(rendered.contains("+B=3"));
        assert!(rendered.contains("+C=4"));
    }

    #[test]
    fn binary_detection() {
        assert!(is_binary(&[0x00, 0x01, 0x02]));
        assert!(is_binary(&[0xff, 0xfe, 0xfd]));
        assert!(!is_binary(b"DATABASE_URL=postgres://localhost\n"));
    }
}

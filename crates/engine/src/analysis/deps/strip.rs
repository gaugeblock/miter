//! Removing literals and comments before a line is parsed for crate roots.
//!
//! A fixture such as `let source = "use storypole_core::x;\n";` is *data*, not a reference, but a
//! line-oriented scanner cannot tell the difference - which is how `miter deps` reported
//! `miter-engine -> outfeed` from its own test fixtures (`GB-199`). Everything that looks like a crate
//! root is therefore searched for in [`code_only`], never in the raw line.

/// Lexer state that has to survive across lines: block comments and multi-line raw strings.
#[derive(Debug, Default)]
pub(super) struct StripState {
    in_block_comment: bool,
    raw_hashes: Option<usize>,
}

/// Returns `line` with comments and string literals removed.
///
/// Deliberately conservative: an unterminated raw string or block comment swallows the rest of the
/// file rather than guessing, because guessing is what produced the false edges.
pub(super) fn code_only(line: &str, state: &mut StripState) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;

    if let Some(hashes) = state.raw_hashes {
        match find_raw_end(&chars, 0, hashes) {
            Some(end) => {
                index = end;
                state.raw_hashes = None;
            }
            None => return String::new(),
        }
    }
    if state.in_block_comment {
        match find_seq(&chars, index, "*/") {
            Some(end) => {
                index = end + 2;
                state.in_block_comment = false;
            }
            None => return String::new(),
        }
    }

    while index < chars.len() {
        if starts_with(&chars, index, "//") {
            break;
        }
        if starts_with(&chars, index, "/*") {
            match find_seq(&chars, index + 2, "*/") {
                Some(end) => {
                    index = end + 2;
                    continue;
                }
                None => {
                    state.in_block_comment = true;
                    break;
                }
            }
        }
        if chars[index] == '"' {
            index = skip_string(&chars, index + 1);
            out.push(' ');
            continue;
        }
        if chars[index] == 'r' {
            let mut probe = index + 1;
            let mut hashes = 0usize;
            while probe < chars.len() && chars[probe] == '#' {
                hashes += 1;
                probe += 1;
            }
            if probe < chars.len() && chars[probe] == '"' {
                match find_raw_end(&chars, probe + 1, hashes) {
                    Some(end) => {
                        index = end;
                        out.push(' ');
                        continue;
                    }
                    None => {
                        state.raw_hashes = Some(hashes);
                        return out;
                    }
                }
            }
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

/// End index just past the closing quote of an ordinary string, honouring backslash escapes.
fn skip_string(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() {
        if chars[index] == '\\' {
            index += 2;
            continue;
        }
        if chars[index] == '"' {
            return index + 1;
        }
        index += 1;
    }
    chars.len()
}

/// End index just past `"` followed by `hashes` `#` characters.
fn find_raw_end(chars: &[char], from: usize, hashes: usize) -> Option<usize> {
    let mut index = from;
    while index < chars.len() {
        if chars[index] == '"' {
            let mut probe = index + 1;
            let mut seen = 0usize;
            while probe < chars.len() && chars[probe] == '#' && seen < hashes {
                seen += 1;
                probe += 1;
            }
            if seen == hashes {
                return Some(probe);
            }
        }
        index += 1;
    }
    None
}

fn starts_with(chars: &[char], index: usize, pattern: &str) -> bool {
    pattern
        .chars()
        .enumerate()
        .all(|(offset, expected)| chars.get(index + offset) == Some(&expected))
}

fn find_seq(chars: &[char], from: usize, pattern: &str) -> Option<usize> {
    let mut index = from;
    while index < chars.len() {
        if starts_with(chars, index, pattern) {
            return Some(index);
        }
        index += 1;
    }
    None
}

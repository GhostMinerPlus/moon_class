use error_stack::ResultExt;

use crate::err;

pub fn find_pat_ignoring_string(pat: &str, s: &str) -> err::Result<Option<usize>> {
    let mut pos = 0;

    while pos < s.len() {
        if s[pos..].starts_with(pat) {
            return Ok(Some(pos));
        } else if s[pos..].starts_with('\"') {
            pos += 1 + find_string_end(&s[pos + 1..])
                .ok_or(err::Error::SyntaxError)
                .attach_printable_lazy(|| {
                    format!("{}: expected '\"', but not found!", &s[pos..])
                })?;
        } else if s[pos..].starts_with('<') {
            pos += 1 + find_angle_end(&s[pos + 1..], "<", ">")?
                .ok_or(err::Error::SyntaxError)
                .attach_printable_lazy(|| format!("{}: expected '>', but not found!", &s[pos..]))?;
        }

        pos += 1;
    }

    Ok(None)
}

/// ''
pub fn find_string_end(s: &str) -> Option<usize> {
    let mut pos = 0;

    while pos < s.len() {
        if s[pos..].starts_with('\"') {
            return Some(pos);
        } else if s[pos..].starts_with('\\') {
            pos += 1;
        }

        pos += 1;
    }

    None
}

pub fn r_find_string_start(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }

    log::debug!("r_find_string_start: {s}");

    let mut pos = s.len() - 1;

    loop {
        if s[pos..].starts_with('\"') && (pos == 0 || !s[pos - 1..].starts_with('\\')) {
            return Some(s.len() - 1 - pos);
        }

        if pos == 0 {
            break;
        }

        pos -= 1;
    }

    None
}

/// <>
pub fn find_angle_end(s: &str, left: &str, right: &str) -> err::Result<Option<usize>> {
    let mut pos = 0;
    let mut depth = 1;

    while pos < s.len() {
        if s[pos..].starts_with(right) {
            depth -= 1;

            if depth == 0 {
                return Ok(Some(pos));
            }
        } else if s[pos..].starts_with(left) {
            depth += 1;
        } else if s[pos..].starts_with('\"') {
            pos += 1 + match find_string_end(&s[pos + 1..]) {
                Some(end) => end,
                None => {
                    return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                        format!("{}: expected '\"', but not found!", &s[pos..])
                    });
                }
            };
        }

        pos += 1;
    }

    Ok(None)
}

/// <>
pub fn r_find_angle_start(s: &str, left: &str, right: &str) -> err::Result<Option<usize>> {
    if s.is_empty() {
        return Ok(None);
    }

    let mut pos = s.len() - 1;
    let mut depth = 1;

    loop {
        if s[pos..].starts_with(left) {
            depth -= 1;

            if depth == 0 {
                return Ok(Some(s.len() - 1 - pos));
            }
        } else if s[pos..].starts_with(right) {
            depth += 1;
        } else if s[pos..].starts_with('\"') && (pos == 0 || !s[pos - 1..].starts_with('\\')) {
            pos -= 1 + match r_find_string_start(&s[..pos]) {
                Some(start) => start,
                None => {
                    return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                        format!("{}: expected '\"', but not found!", &s[..pos + 1])
                    });
                }
            };
        }

        if pos == 0 {
            break;
        }

        pos -= 1;
    }

    Ok(None)
}

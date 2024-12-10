use std::pin::Pin;

use error_stack::ResultExt;

use crate::{err, AsClassManager, Fu};

use super::{
    inc,
    inner::unwrap_value,
    string::{find_angle_end, find_string_end},
};

pub fn script<'a, 'a1, 'f, CM>(
    ce: &'a mut CM,
    s: &'a1 str,
) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
where
    'a: 'f,
    'a1: 'f,
    CM: AsClassManager,
{
    Box::pin(async move {
        let mut pos = 0;

        let mut out_s = s.to_string();

        while pos < s.len() {
            if s[pos..].starts_with("${") {
                let end = s[pos + 2..].find('}').unwrap() + pos + 2;

                let v_s = &s[pos + 2..end];

                let v = unwrap_value(ce, &inc::IncVal::from_str(v_s)?).await?;

                out_s = out_s.replace(&s[pos..end + 1], &v.join("\n"));
            } else if s[pos..].starts_with('\"') {
                pos += 1 + match find_string_end(&s[pos + 1..]) {
                    Some(end) => end,
                    None => {
                        return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                            format!("{}: expected '\"', but not found!", &s[pos..])
                        });
                    }
                };
            } else if s[pos..].starts_with('<') {
                pos += 1 + find_angle_end(&s[pos + 1..], "<", ">")?
                    .ok_or(err::Error::SyntaxError)
                    .attach_printable_lazy(|| {
                        format!("{}: expected '>', but not found!", &s[pos..])
                    })?;
            }

            pos += 1;
        }

        Ok(vec![out_s])
    })
}

pub fn object<'a, 'a1, 'f, CM>(
    ce: &'a mut CM,
    s: &'a1 str,
) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
where
    'a: 'f,
    'a1: 'f,
    CM: AsClassManager,
{
    Box::pin(async move {
        if s.starts_with('[') {
            let mut obj_s_v = vec![];
            let mut pos = 1;
            let mut start = pos;

            while pos < s.len() {
                if s[pos..].starts_with(',') {
                    obj_s_v.push(s[start..pos].trim());

                    start = pos + 1;
                } else if s[pos..].starts_with(']') {
                    let last = s[start..pos].trim();

                    if !last.is_empty() {
                        obj_s_v.push(last);
                    }

                    break;
                } else if s[pos..].starts_with('[') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "[", "]")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '[', but not found!", &s[pos..])
                        })?;
                } else if s[pos..].starts_with('{') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "{", "}")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '{{', but not found!", &s[pos..])
                        })?;
                } else if s[pos..].starts_with('\"') {
                    pos += 1 + match find_string_end(&s[pos + 1..]) {
                        Some(end) => end,
                        None => {
                            return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                                format!("{}: expected '\"', but not found!", &s[pos..])
                            });
                        }
                    };
                } else if s[pos..].starts_with('<') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "<", ">")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '>', but not found!", &s[pos..])
                        })?;
                }

                pos += 1;
            }

            let iv_res_v = obj_s_v
                .iter()
                .map(|s| inc::IncVal::from_str(s))
                .collect::<Vec<err::Result<inc::IncVal>>>();

            let mut rs = Vec::with_capacity(iv_res_v.len());

            for iv in iv_res_v {
                rs.extend(unwrap_value(ce, &iv?).await?);
            }

            Ok(rs)
        } else if s.ends_with('}') {
            let (mut pos, root) = if s.starts_with('@') {
                let pos = s.find('{').unwrap() + 1;
                let root = s[1..pos - 1].to_string();

                log::debug!("unwrap_value: root = {root}");

                (pos, root)
            } else {
                (1, uuid::Uuid::new_v4().to_string())
            };
            let mut entry_v = vec![];
            let mut start = pos;

            while pos < s.len() {
                if s[pos..].starts_with(',') {
                    entry_v.push(s[start..pos].trim());

                    start = pos + 1;
                } else if s[pos..].starts_with('}') {
                    let last = s[start..pos].trim();

                    if !last.is_empty() {
                        entry_v.push(last);
                    }

                    break;
                } else if s[pos..].starts_with('[') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "[", "]")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '[', but not found!", &s[pos..])
                        })?;
                } else if s[pos..].starts_with('{') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "{", "}")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '{{', but not found!", &s[pos..])
                        })?;
                } else if s[pos..].starts_with('\"') {
                    pos += 1 + match find_string_end(&s[pos + 1..]) {
                        Some(end) => end,
                        None => {
                            return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                                format!("{}: expected '\"', but not found!", &s[pos..])
                            });
                        }
                    };
                } else if s[pos..].starts_with('<') {
                    pos += 1 + find_angle_end(&s[pos + 1..], "<", ">")?
                        .ok_or(err::Error::SyntaxError)
                        .attach_printable_lazy(|| {
                            format!("{}: expected '>', but not found!", &s[pos..])
                        })?;
                }

                pos += 1;
            }

            for entry in entry_v {
                let mut pos = 0;

                while pos < entry.len() {
                    if entry[pos..].starts_with(':') {
                        break;
                    } else if entry[pos..].starts_with('[') {
                        pos += 1 + find_angle_end(&entry[pos + 1..], "[", "]")?
                            .ok_or(err::Error::SyntaxError)
                            .attach_printable_lazy(|| {
                                format!("{}: expected '[', but not found!", &entry[pos..])
                            })?;
                    } else if entry[pos..].starts_with('{') {
                        pos += 1 + find_angle_end(&entry[pos + 1..], "{", "}")?
                            .ok_or(err::Error::SyntaxError)
                            .attach_printable_lazy(|| {
                                format!("{}: expected '{{', but not found!", &entry[pos..])
                            })?;
                    } else if entry[pos..].starts_with('\"') {
                        pos += 1 + match find_string_end(&entry[pos + 1..]) {
                            Some(end) => end,
                            None => {
                                return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                                    format!("{}: expected '\"', but not found!", &entry[pos..])
                                });
                            }
                        };
                    } else if entry[pos..].starts_with('<') {
                        pos += 1 + find_angle_end(&entry[pos + 1..], "<", ">")?
                            .ok_or(err::Error::SyntaxError)
                            .attach_printable_lazy(|| {
                                format!("{}: expected '>', but not found!", &entry[pos..])
                            })?;
                    }

                    pos += 1;
                }

                let key = unwrap_value(ce, &inc::IncVal::from_str(entry[0..pos].trim())?).await?;
                let value_v =
                    unwrap_value(ce, &inc::IncVal::from_str(entry[pos + 1..].trim())?).await?;

                if s.starts_with('@') {
                    ce.remove(
                        key.first().unwrap(),
                        &root,
                        ce.get(key.first().unwrap(), &root).await?,
                    )
                    .await?;
                }

                ce.append(key.first().unwrap(), &root, value_v).await?;
            }

            Ok(vec![root])
        } else {
            Err(err::Error::SyntaxError).attach_printable_lazy(|| format!("{s} not a object!"))
        }
    })
}

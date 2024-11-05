use std::fmt::Display;

use error_stack::ResultExt;

use crate::err;

pub fn find_pat_ignoring_string(pat: &str, s: &str) -> err::Result<Option<usize>> {
    let mut pos = 0;

    loop {
        if s[pos..].starts_with(pat) {
            break;
        } else if s[pos..].starts_with('\'') {
            pos += 1;

            loop {
                if s[pos..].starts_with('\'') {
                    pos += 1;

                    break;
                } else if s[pos..].starts_with('\\') {
                    pos += 2;
                } else {
                    pos += 1;
                }

                if pos >= s.len() {
                    return Err(err::Error::SyntaxError)
                        .attach_printable_lazy(|| format!("{s}: expected '\'', but not found!"));
                }
            }
        } else {
            pos += 1;
        }

        if pos >= s.len() {
            return Ok(None);
        }
    }

    Ok(Some(pos))
}

pub fn str_of_value(word: &str) -> String {
    let content = word
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\'", "\\'");

    if content.len() > word.len()
        || content.contains('[')
        || content.contains(']')
        || content.contains('=')
        || content.contains(';')
        || content.contains('?')
        || content.contains(' ')
    {
        format!("'{content}'")
    } else {
        word.to_string()
    }
}

pub fn value_of_str(mut word: &str) -> String {
    if !word.starts_with('\'') {
        return word.to_string();
    }

    word = &word[1..word.len() - 1];

    let mut rs = String::new();
    let mut pos = 0;
    while pos < word.len() {
        pos += match word[pos..].find('\\') {
            Some(offset) => {
                let ch = &word[pos + offset + 1..pos + offset + 2];
                let ch = match ch {
                    "n" => "\n",
                    "t" => "\t",
                    _ => ch,
                };
                rs = format!("{rs}{}{ch}", &word[pos..pos + offset]);
                offset + 2
            }
            None => {
                rs = format!("{rs}{}", &word[pos..]);
                break;
            }
        };
    }
    rs
}

pub fn rs_2_str(rs: &[String]) -> String {
    let mut acc = String::new();

    if rs.is_empty() {
        return acc;
    }

    for i in 0..rs.len() - 1 {
        let item = &rs[i];

        acc = if item.ends_with("\\c") {
            format!("{acc}{}", &item[0..item.len() - 2])
        } else {
            format!("{acc}{item}\n")
        }
    }

    let item = rs.last().unwrap();

    acc = if item.ends_with("\\c") {
        format!("{acc}{}", &item[0..item.len() - 2])
    } else {
        format!("{acc}{item}")
    };

    acc
}

pub fn str_2_rs(s: &str) -> Vec<String> {
    let mut rs = Vec::new();

    for line in s.lines() {
        if line.len() > 500 {
            let mut start = 0;

            loop {
                let end = start + 500;

                if end >= line.len() {
                    rs.push(line[start..].to_string());

                    break;
                }

                rs.push(format!("{}\\c", &line[start..end]));

                start = end;
            }
        } else {
            rs.push(line.to_string());
        }
    }

    rs
}

pub enum IncVal {
    Value(String),
    Addr((Box<IncVal>, Box<IncVal>)),
}

impl IncVal {
    /// new('view(main)')(new('view(main)'))
    pub fn from_str(s: &str) -> err::Result<Self> {
        if s.is_empty() {
            return Ok(IncVal::Value(String::new()));
        }

        let mut pos = s.len() - 1;
        let mut depth = 0;

        loop {
            if s[pos..].starts_with(']') {
                depth += 1;
            } else if s[pos..].starts_with('[') {
                depth -= 1;

                if depth == 0 {
                    let class = IncVal::from_str(s[0..pos].trim())?;
                    let source = IncVal::from_str(s[pos + 1..s.len() - 1].trim())?;

                    return Ok(Self::Addr((Box::new(class), Box::new(source))));
                }
            } else if s[pos..].starts_with('\'') && (pos == 0 || !s[pos - 1..].starts_with('\\')) {
                pos -= 1;

                loop {
                    if s[pos..].starts_with('\'') && (pos == 0 || !s[pos - 1..].starts_with('\\')) {
                        break;
                    }

                    if pos == 0 {
                        return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                            format!("{s}: expected '\'', but not found!")
                        });
                    }

                    pos -= 1;
                }
            }

            if pos == 0 {
                break;
            }

            pos -= 1;
        }

        if s == "?" {
            Ok(IncVal::Value(uuid::Uuid::new_v4().to_string()))
        } else {
            Ok(IncVal::Value(value_of_str(s)))
        }
    }

    pub fn as_value(&self) -> Option<&String> {
        match self {
            IncVal::Value(v) => Some(v),
            IncVal::Addr(_) => None,
        }
    }

    pub fn as_addr(&self) -> Option<(&IncVal, &IncVal)> {
        match self {
            IncVal::Value(_) => None,
            IncVal::Addr((class, source)) => Some((class, source)),
        }
    }
}

impl Display for IncVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncVal::Value(v) => write!(f, "{}", str_of_value(v)),
            IncVal::Addr((class, source)) => write!(f, "{class}[{source}]"),
        }
    }
}

pub enum Opt {
    Append,
    Set,
}

impl Display for Opt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opt::Append => write!(f, "+="),
            Opt::Set => write!(f, "="),
        }
    }
}

pub struct Inc {
    target: IncVal,
    operator: Opt,
    class: IncVal,
    source: IncVal,
}

impl Inc {
    pub fn class(&self) -> &IncVal {
        &self.class
    }

    pub fn source(&self) -> &IncVal {
        &self.source
    }

    pub fn target(&self) -> &IncVal {
        &self.target
    }

    pub fn operator(&self) -> &Opt {
        &self.operator
    }

    /// new('view(main)'), ?
    pub fn from_str(s: &str) -> err::Result<Self> {
        let pos = find_pat_ignoring_string("=", s)?
            .ok_or(err::Error::NotFound)
            .attach_printable("expected '=', but not found!")?;

        let (target, operator) = if s[pos - 1..].starts_with("+=") {
            (IncVal::from_str(s[0..pos - 1].trim())?, Opt::Append)
        } else {
            (IncVal::from_str(s[0..pos].trim())?, Opt::Set)
        };

        let (class, source) = match IncVal::from_str(s[pos + 1..].trim())? {
            IncVal::Value(_) => {
                return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                    format!("'{}' need a source but not found!", &s[pos + 1..].trim())
                });
            }
            IncVal::Addr((class, source)) => (*class, *source),
        };

        Ok(Self {
            target,
            operator,
            class,
            source,
        })
    }
}

impl Display for Inc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}[{}]",
            self.target, self.operator, self.class, self.source
        )
    }
}

pub fn inc_v_from_str(mut s: &str) -> err::Result<Vec<Inc>> {
    let mut inc_v = vec![];

    while let Some(pos) = find_pat_ignoring_string(";", s)? {
        inc_v.push(Inc::from_str(s[0..pos].trim())?);

        s = &s[pos + 1..];
    }

    Ok(inc_v)
}

pub fn inc_v_to_string(inc_v: &[Inc]) -> String {
    let mut s = String::new();

    for inc in inc_v {
        s = format!("{s}{inc};\n")
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let inc = Inc::from_str("test = new['view[main]']").unwrap();

        assert_eq!(inc.class().as_value().unwrap(), "new");
    }

    #[test]
    fn test_display() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let inc = Inc::from_str("test = new['view[main]']").unwrap();

        assert_eq!(inc.to_string(), "test = new['view[main]']");
    }

    #[test]
    fn test_inc_v() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let inc_v = inc_v_from_str("test = new['view[main]'];test = new['view[main]'];").unwrap();

        assert_eq!(
            inc_v_to_string(&inc_v),
            "test = new['view[main]'];\ntest = new['view[main]'];\n"
        )
    }
}

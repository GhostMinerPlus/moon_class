use std::fmt::Display;

use error_stack::ResultExt;

use crate::{
    err,
    util::{str_of_value, value_of_str},
};

use super::string::{
    find_angle_end, find_pat_ignoring_string, find_string_end, r_find_angle_start,
    r_find_string_start,
};

#[derive(Debug)]
pub enum IncVal {
    /// xxx, ''
    Value(String),
    /// x(x)
    Addr((Box<IncVal>, Box<IncVal>)),
    /// {}, []
    Object(String),
    ///  <>
    Script(String),
}

impl IncVal {
    /// new('view(main)')(new('view(main)'))
    pub fn from_str(s: &str) -> err::Result<Self> {
        if s.is_empty() {
            return Ok(IncVal::Value(String::new()));
        }

        if ((s.starts_with('{') || s.starts_with("@")) && s.ends_with('}'))
            || s.starts_with('[') && s.ends_with(']')
        {
            if s.starts_with("@{") {
                return Ok(IncVal::Object(format!(
                    "@{}{}",
                    uuid::Uuid::new_v4(),
                    &s[1..]
                )));
            } else {
                return Ok(IncVal::Object(s.to_string()));
            }
        }

        if s.starts_with('<') && s.ends_with('>') {
            return Ok(IncVal::Script(s[1..s.len() - 1].to_string()));
        }

        let mut pos = s.len() - 1;
        let mut depth = 0;

        loop {
            if s[pos..].starts_with(')') {
                depth += 1;
            } else if s[pos..].starts_with('(') {
                depth -= 1;

                if depth == 0 {
                    let class = IncVal::from_str(s[..pos].trim())?;
                    let source = IncVal::from_str(s[pos + 1..s.len() - 1].trim())?;

                    return Ok(Self::Addr((Box::new(class), Box::new(source))));
                }
            } else if s[pos..].starts_with('\"') && (pos == 0 || !s[pos - 1..].starts_with('\\')) {
                pos -= 1 + r_find_string_start(&s[..pos])
                    .ok_or(err::Error::SyntaxError)
                    .attach_printable_lazy(|| {
                        format!("{}: expected '\"' at 0, but not found!", &s[..pos + 1])
                    })?;
            } else if s[pos..].starts_with('>') {
                pos -= 1 + r_find_angle_start(&s[..pos], "<", ">")?
                    .ok_or(err::Error::SyntaxError)
                    .attach_printable_lazy(|| {
                        format!("{}: expected '<' at 0, but not found!", &s[..pos + 1])
                    })?;
            }

            if pos == 0 {
                break;
            }

            pos -= 1;
        }

        Ok(IncVal::Value(value_of_str(s)))
    }

    pub fn as_value(&self) -> Option<&String> {
        match self {
            IncVal::Value(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_addr(&self) -> Option<(&IncVal, &IncVal)> {
        match self {
            IncVal::Addr((class, source)) => Some((class, source)),
            _ => None,
        }
    }
}

impl Display for IncVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncVal::Value(v) => write!(f, "{}", str_of_value(v)),
            IncVal::Addr((class, source)) => write!(f, "{class}({source})"),
            IncVal::Object(s) => write!(f, "{s}"),
            IncVal::Script(s) => write!(f, "<{s}>"),
        }
    }
}

#[derive(Debug)]
pub enum Opt {
    Append,
    Set,
}

impl Display for Opt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opt::Append => write!(f, "="),
            Opt::Set => write!(f, ":="),
        }
    }
}

#[derive(Debug)]
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
        let mut pos = 0;

        let mut target_op = None;
        let mut operator_op = None;

        while pos < s.len() {
            if s[pos..].starts_with(":=") {
                log::debug!("from_str: {s}");

                target_op = Some(IncVal::from_str(s[..pos].trim())?);
                operator_op = Some(Opt::Set);

                pos += 1;
                break;
            } else if s[pos..].starts_with("=") {
                target_op = Some(IncVal::from_str(s[..pos].trim())?);
                operator_op = Some(Opt::Append);
                break;
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

        let (class, source) = match IncVal::from_str(s[pos + 1..].trim())? {
            IncVal::Addr((class, source)) => (*class, *source),
            _ => {
                return Err(err::Error::SyntaxError).attach_printable_lazy(|| {
                    format!("'{}' need a source but not found!", &s[pos + 1..].trim())
                });
            }
        };

        Ok(Self {
            target: target_op.unwrap(),
            operator: operator_op.unwrap(),
            class,
            source,
        })
    }
}

impl Display for Inc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}({})",
            self.target, self.operator, self.class, self.source
        )
    }
}

pub fn inc_v_from_str(mut s: &str) -> err::Result<Vec<Inc>> {
    let mut inc_v = vec![];

    while let Some(pos) = find_pat_ignoring_string(";", s)? {
        inc_v.push(Inc::from_str(s[..pos].trim())?);

        s = &s[pos + 1..];
    }

    Ok(inc_v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inc_v_to_string(inc_v: &[Inc]) -> String {
        let mut s = String::new();

        for inc in inc_v {
            s = format!("{s}{inc};\n")
        }

        s
    }

    #[test]
    fn test() {
        let inc = Inc::from_str("test = new('view(main)')").unwrap();

        assert_eq!(inc.class().as_value().unwrap(), "new");
    }

    #[test]
    fn test_display() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let inc = Inc::from_str("test := new(\"view(main)\")").unwrap();

        assert_eq!(inc.to_string(), "\"test\" := \"new\"(\"view(main)\")");
    }

    #[test]
    fn test_inc_v() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let inc_v =
            inc_v_from_str("test = new(\"view(main)\");[{<test;=(>}] = new(\"view(main)\");")
                .unwrap();

        assert_eq!(
            inc_v_to_string(&inc_v),
            "\"test\" = \"new\"(\"view(main)\");\n[{<test;=(>}] = \"new\"(\"view(main)\");\n"
        )
    }
}

use error_stack::ResultExt;

use crate::err;

mod class {
    use super::Class;

    pub fn parse_class_v(s: &str) -> Vec<Box<Class>> {
        let mut depth = 0;
        let mut start = 0;
        let mut content_v = vec![];
        let mut pos = 0;

        while pos < s.len() {
            if s[pos..].starts_with('<') {
                depth += 1;
            } else if s[pos..].starts_with('>') {
                depth -= 1;
            } else if s[pos..].starts_with(',') && depth == 0 {
                content_v.push(Class::from_str(s[start..pos].trim()));
                start = pos + 1;
            } else if s[pos..].starts_with('\'') {
                pos += 1;

                loop {
                    if s[pos..].starts_with('\'') {
                        break;
                    } else if s[pos..].starts_with('\\') {
                        pos += 2;
                    } else {
                        pos += 1;
                    }

                    if pos >= s.len() {
                        panic!("unclosed string!");
                    }
                }
            }

            pos += 1;
        }

        content_v.push(Class::from_str(s[start..].trim()));

        content_v
    }
}

pub fn find_pat_ignoring_string(pat: &str, s: &str) -> Option<usize> {
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
                    panic!("unclosed string!");
                }
            }
        } else {
            pos += 1;
        }

        if pos >= s.len() {
            return None;
        }
    }

    Some(pos)
}

pub fn unescape_word(word: &str) -> String {
    let content = word
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\'", "\\'");

    if content.len() > word.len()
        || content.contains(',')
        || content.contains('<')
        || content.contains('>')
    {
        format!("'{content}'")
    } else {
        word.to_string()
    }
}

pub fn escape_word(mut word: &str) -> String {
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

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Class {
    pub name: String,
    pub left_op: Option<Box<Class>>,
    pub right_op: Option<Box<Class>>,
}

impl Class {
    pub fn new(name: &str, left_op: Option<Box<Class>>, right_op: Option<Box<Class>>) -> Box<Self> {
        Box::new(Self {
            name: escape_word(&name),
            left_op,
            right_op,
        })
    }

    pub fn new_with_name(name: &str) -> Box<Self> {
        Box::new(Self {
            name: escape_word(&name),
            left_op: None,
            right_op: None,
        })
    }

    pub fn pair(&self) -> err::Result<String> {
        let left = self
            .left_op
            .as_ref()
            .ok_or(err::Error::NotFound)
            .attach_printable("no left!")?;
        let right = self
            .left_op
            .as_ref()
            .ok_or(err::Error::NotFound)
            .attach_printable("no right!")?;

        if !left.is_tag() || !right.is_tag() {
            return Err(err::Error::RuntimeError).attach_printable("not a final class");
        }

        Ok(format!(
            "{}, {}",
            unescape_word(&left.name),
            unescape_word(&right.name)
        ))
    }

    pub fn is_final_or_tag(&self) -> bool {
        (self.left_op.is_none() || self.left_op.as_ref().unwrap().is_tag())
            && (self.right_op.is_none() || self.right_op.as_ref().unwrap().is_tag())
    }

    pub fn is_tag(&self) -> bool {
        self.left_op.is_none() && self.right_op.is_none()
    }

    pub fn to_string(&self) -> String {
        if self.is_tag() {
            return unescape_word(&self.name);
        }

        format!(
            "{}<{}, {}>",
            self.name,
            match &self.left_op {
                Some(item) => item.to_string(),
                None => String::new(),
            },
            match &self.right_op {
                Some(item) => item.to_string(),
                None => String::new(),
            }
        )
    }

    pub fn from_str(s: &str) -> Box<Class> {
        if s.starts_with('\'') {
            return Box::new(Self {
                name: escape_word(s),
                left_op: None,
                right_op: None,
            });
        }

        if let Some(pos) = find_pat_ignoring_string("<", s) {
            let mut content_v = class::parse_class_v(&s[pos + 1..s.len() - 1]);

            let mut origin = Box::new(Self {
                name: s[0..pos].to_string(),
                left_op: if !content_v.is_empty() {
                    Some(content_v.remove(0))
                } else {
                    None
                },
                right_op: if !content_v.is_empty() {
                    Some(content_v.remove(0))
                } else {
                    None
                },
            });

            for item in content_v {
                origin = Box::new(Self {
                    name: origin.name.clone(),
                    left_op: Some(origin),
                    right_op: Some(item),
                })
            }

            origin
        } else {
            Box::new(Self {
                name: s.to_string(),
                left_op: None,
                right_op: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let class = Class::from_str("test<test, test<test, test>>");

        assert_eq!(class.left_op.unwrap().name, "test")
    }

    #[test]
    fn test_string() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let class = Class::from_str("test<test, 'test '>");

        assert_eq!(class.right_op.unwrap().name, "test ")
    }
}

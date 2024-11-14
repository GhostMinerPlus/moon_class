pub mod executor;

pub fn str_of_value(word: &str) -> String {
    let content = word
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\"", "\\\"");

    format!("\"{content}\"")
}

pub fn value_of_str(mut word: &str) -> String {
    if word.starts_with('<') {
        return word[1..word.len() - 1].to_string();
    }

    if word.starts_with('\"') {
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
    } else {
        word.to_string()
    }
}

pub fn rs_2_str(rs: &[String]) -> String {
    let mut acc = String::new();

    if rs.is_empty() {
        return acc;
    }

    for i in 0..rs.len() - 1 {
        let item = &rs[i];

        acc = if item.ends_with("\\c") {
            format!("{acc}{}", &item[..item.len() - 2])
        } else {
            format!("{acc}{item}\n")
        }
    }

    let item = rs.last().unwrap();

    acc = if item.ends_with("\\c") {
        format!("{acc}{}", &item[..item.len() - 2])
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

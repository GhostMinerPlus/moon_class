use std::{collections::HashSet, pin::Pin};

use inc::inc_v_from_str;

use crate::{err, AsClassManager, ClassManager, Fu};

use super::str_2_rs;

mod string;
mod inc {
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
                } else if s[pos..].starts_with('\"')
                    && (pos == 0 || !s[pos - 1..].starts_with('\\'))
                {
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
            let _ = env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("debug"),
            )
            .is_test(true)
            .try_init();

            let inc = Inc::from_str("test := new(\"view(main)\")").unwrap();

            assert_eq!(inc.to_string(), "test := new(\"view(main)\")");
        }

        #[test]
        fn test_inc_v() {
            let _ = env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("debug"),
            )
            .is_test(true)
            .try_init();

            let inc_v =
                inc_v_from_str("test = new(\"view(main)\");[{<test;=(>}] = new(\"view(main)\");")
                    .unwrap();

            assert_eq!(
                inc_v_to_string(&inc_v),
                "test = new(\"view(main)\");\n[{<test;=(>}] = new(\"view(main)\");\n"
            )
        }
    }
}
mod inner {
    use std::pin::Pin;

    use error_stack::ResultExt;

    use crate::{
        err,
        util::executor::{
            string::{find_angle_end, find_string_end},
            ClassExecutor,
        },
        AsClassManager, Fu,
    };

    use super::inc::IncVal;

    pub fn unwrap_value<'a, 'a1, 'f, CM: AsClassManager + Send + Sync>(
        ce: &'a mut ClassExecutor<'_, CM>,
        inc_val: &'a1 IncVal,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            match inc_val {
                IncVal::Object(s) => {
                    if s.starts_with('[') {
                        let mut obj_s_v = vec![];
                        let mut pos = 1;
                        let mut start = pos;

                        while pos < s.len() {
                            if s[pos..].starts_with(',') || s[pos..].starts_with(']') {
                                obj_s_v.push(s[start..pos].trim());

                                start = pos + 1;
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
                                        return Err(err::Error::SyntaxError).attach_printable_lazy(
                                            || {
                                                format!(
                                                    "{}: expected '\"', but not found!",
                                                    &s[pos..]
                                                )
                                            },
                                        );
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
                            .map(|s| IncVal::from_str(s))
                            .collect::<Vec<err::Result<IncVal>>>();

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
                                        return Err(err::Error::SyntaxError).attach_printable_lazy(
                                            || {
                                                format!(
                                                    "{}: expected '\"', but not found!",
                                                    &s[pos..]
                                                )
                                            },
                                        );
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
                                            format!(
                                                "{}: expected '[', but not found!",
                                                &entry[pos..]
                                            )
                                        })?;
                                } else if entry[pos..].starts_with('{') {
                                    pos += 1 + find_angle_end(&entry[pos + 1..], "{", "}")?
                                        .ok_or(err::Error::SyntaxError)
                                        .attach_printable_lazy(|| {
                                            format!(
                                                "{}: expected '{{', but not found!",
                                                &entry[pos..]
                                            )
                                        })?;
                                } else if entry[pos..].starts_with('\"') {
                                    pos += 1 + match find_string_end(&entry[pos + 1..]) {
                                        Some(end) => end,
                                        None => {
                                            return Err(err::Error::SyntaxError)
                                                .attach_printable_lazy(|| {
                                                    format!(
                                                        "{}: expected '\"', but not found!",
                                                        &entry[pos..]
                                                    )
                                                });
                                        }
                                    };
                                } else if entry[pos..].starts_with('<') {
                                    pos += 1 + find_angle_end(&entry[pos + 1..], "<", ">")?
                                        .ok_or(err::Error::SyntaxError)
                                        .attach_printable_lazy(|| {
                                            format!(
                                                "{}: expected '>', but not found!",
                                                &entry[pos..]
                                            )
                                        })?;
                                }

                                pos += 1;
                            }

                            let key =
                                unwrap_value(ce, &IncVal::from_str(entry[0..pos].trim())?).await?;
                            let value_v =
                                unwrap_value(ce, &IncVal::from_str(entry[pos + 1..].trim())?)
                                    .await?;

                            if s.starts_with('@') {
                                ce.clear(key.first().unwrap(), &root).await?;
                            }

                            ce.append(key.first().unwrap(), &root, value_v).await?;
                        }

                        Ok(vec![root])
                    } else {
                        Err(err::Error::SyntaxError)
                            .attach_printable_lazy(|| format!("{s} not a object!"))
                    }
                }
                IncVal::Script(v) => Ok(vec![v.clone()]),
                IncVal::Value(v) => Ok(vec![v.clone()]),
                IncVal::Addr((class, source)) => {
                    let class_v = unwrap_value(ce, class).await?;
                    let source_v = unwrap_value(ce, source).await?;
                    let mut rs = vec![];

                    for class in &class_v {
                        for source in &source_v {
                            rs.extend(ce.get(class, source).await?);
                        }
                    }

                    Ok(rs)
                }
            }
        })
    }
}

pub struct ClassExecutor<'cm, CM: AsClassManager> {
    global_cm: &'cm mut CM,
    temp_cm: ClassManager,
}

impl<'cm, CM: AsClassManager> ClassExecutor<'cm, CM> {
    pub fn execute<'a, 'a1, 'f>(
        &'a mut self,
        inc_v: &'a1 [inc::Inc],
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            for inc in inc_v {
                let class_v = inner::unwrap_value(self, inc.class()).await?;
                let source_v = inner::unwrap_value(self, inc.source()).await?;
                let target_v = inner::unwrap_value(self, inc.target()).await?;

                match inc.operator() {
                    inc::Opt::Append => {
                        for class in &class_v {
                            for source in &source_v {
                                self.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                    inc::Opt::Set => {
                        for class in &class_v {
                            for source in &source_v {
                                self.clear(class, source).await?;

                                self.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                }
            }

            self.get("$result", "").await
        })
    }

    pub fn execute_script<'a, 'a1, 'f>(
        &'a mut self,
        script: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let inc_v = inc::inc_v_from_str(script)?;

            self.execute(&inc_v).await
        })
    }

    pub fn new(global: &'cm mut CM) -> Self {
        Self {
            global_cm: global,
            temp_cm: ClassManager::new(),
        }
    }

    pub fn load_json<'a, 'b, 'c, 'd, 'f>(
        &'a mut self,
        class: &'b str,
        source: &'c str,
        jv: &'d json::JsonValue,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'b: 'f,
        'c: 'f,
        'd: 'f,
    {
        Box::pin(async move {
            match jv {
                json::JsonValue::Object(object) => {
                    let new_source = uuid::Uuid::new_v4().to_string();

                    for (class, sub_jv) in object.iter() {
                        self.load_json(class, &new_source, sub_jv).await?;
                    }

                    self.append(class, source, vec![new_source]).await
                }
                json::JsonValue::Array(vec) => {
                    for sub_jv in vec.iter() {
                        self.load_json(class, source, sub_jv).await?;
                    }

                    Ok(())
                }
                json::JsonValue::Null => Ok(()),
                json::JsonValue::Short(short) => {
                    self.append(class, source, vec![short.to_string()]).await
                }
                json::JsonValue::String(s) => self.append(class, source, vec![s.clone()]).await,
                json::JsonValue::Number(number) => {
                    self.append(class, source, vec![number.to_string()]).await
                }
                json::JsonValue::Boolean(b) => {
                    self.append(class, source, vec![b.to_string()]).await
                }
            }
        })
    }

    pub fn temp_ref(&self) -> &ClassManager {
        &self.temp_cm
    }

    pub fn temp(self) -> ClassManager {
        self.temp_cm
    }

    pub fn dump_json<'a, 'b, 'c, 'f>(
        &'a self,
        class_v: &'b [String],
        source_v: &'c [String],
    ) -> Pin<Box<dyn Fu<Output = err::Result<json::JsonValue>> + 'f>>
    where
        'a: 'f,
        'b: 'f,
        'c: 'f,
    {
        Box::pin(async move {
            let mut rj = json::array![];

            for source in source_v {
                let mut item = json::object! {};

                for class in class_v {
                    let sub_source_v = self.get(class, source).await?;

                    if !sub_source_v.is_empty() {
                        let _ = item.insert(class, self.dump_json(class_v, &sub_source_v).await?);
                    }
                }

                if item.is_empty() {
                    let _ = rj.push(json::JsonValue::String(source.to_string()));
                } else {
                    let _ = rj.push(item);
                }
            }

            Ok(rj)
        })
    }
}

impl<'cm, CM: AsClassManager> AsClassManager for ClassExecutor<'cm, CM> {
    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.get(class, source).await
            } else {
                match class {
                    "+" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let sz = left_v.len();

                        let mut rs = vec![];

                        for i in 0..sz {
                            let left = left_v[i].parse::<f64>().unwrap();
                            let right = right_v[i].parse::<f64>().unwrap();

                            rs.push((left + right).to_string());
                        }

                        Ok(rs)
                    }
                    "-" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let sz = left_v.len();

                        let mut rs = vec![];

                        for i in 0..sz {
                            let left = left_v[i].parse::<f64>().unwrap();
                            let right = right_v[i].parse::<f64>().unwrap();

                            rs.push((left - right).to_string());
                        }

                        Ok(rs)
                    }
                    "*" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let sz = left_v.len();

                        let mut rs = vec![];

                        for i in 0..sz {
                            let left = left_v[i].parse::<f64>().unwrap();
                            let right = right_v[i].parse::<f64>().unwrap();

                            rs.push((left * right).to_string());
                        }

                        Ok(rs)
                    }
                    "/" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let sz = left_v.len();

                        let mut rs = vec![];

                        for i in 0..sz {
                            let left = left_v[i].parse::<f64>().unwrap();
                            let right = right_v[i].parse::<f64>().unwrap();

                            rs.push((left / right).to_string());
                        }

                        Ok(rs)
                    }
                    "#dump" => {
                        let rj = self.temp_cm.dump(source);

                        Ok(str_2_rs(&rj.to_string()))
                    }
                    "#inner" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let mut left_set = HashSet::new();

                        left_set.extend(left_v);

                        let mut rs = vec![];

                        for right_item in right_v {
                            if left_set.contains(&right_item) {
                                rs.push(right_item);
                            }
                        }

                        Ok(rs)
                    }
                    "#source" => {
                        let target_v = self.get("$target", source).await?;
                        let class_v = self.get("$class", source).await?;

                        self.get_source(&target_v[0], &class_v[0]).await
                    }
                    "#if" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        if left_v.is_empty() {
                            Ok(right_v)
                        } else {
                            Ok(left_v)
                        }
                    }
                    "#left" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let mut left_set = HashSet::new();

                        left_set.extend(left_v);

                        for right_item in &right_v {
                            left_set.remove(right_item);
                        }

                        Ok(left_set.into_iter().collect())
                    }
                    _ => self.global_cm.get(class, source).await,
                }
            }
        })
    }

    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.clear(class, source).await
            } else {
                self.global_cm.clear(class, source).await
            }
        })
    }

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                match class {
                    "$switch" => {
                        for target in &target_v {
                            let case_v = self.get("$case", target).await?;

                            if !self
                                .execute_script(case_v.first().unwrap())
                                .await?
                                .is_empty()
                            {
                                let then_v = self.get("$then", target).await?;

                                if let Some(then) = then_v.first() {
                                    self.execute_script(then).await?;
                                }

                                break;
                            }
                        }

                        Ok(())
                    }
                    "$loop" => {
                        let inc_v = inc_v_from_str(target_v.first().unwrap())?;

                        while !self.execute(&inc_v).await?.is_empty() {}

                        Ok(())
                    }
                    _ => self.temp_cm.append(class, source, target_v).await,
                }
            } else {
                self.global_cm.append(class, source, target_v).await
            }
        })
    }

    fn get_source<'a, 'a1, 'a2, 'f>(
        &'a self,
        target: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.get_source(target, class).await
            } else {
                self.global_cm.get_source(target, class).await
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ClassManager;

    use super::*;

    #[test]
    fn test_return() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            log::debug!("start");

            let mut cm = ClassManager::new();

            let rs = ClassExecutor::new(&mut cm)
                .execute_script(
                    "test = test(test);
                    test(test) = $result();",
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "test");
        })
    }

    #[test]
    fn test_add() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            log::debug!("start");

            let mut cm = ClassManager::new();

            let rs = ClassExecutor::new(&mut cm)
                .execute_script(
                    "1 = $left(test);
                    1 = $right(test);
                    +(test) = $result();",
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }

    #[test]
    fn test_json_io() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            log::debug!("start");

            let mut cm = ClassManager::new();

            let mut ce = ClassExecutor::new(&mut cm);

            ce.load_json(
                "$data",
                "",
                &json::object! {
                    "$width": 1024,
                    "$height": 1024
                },
            )
            .await
            .unwrap();

            let rs = ce
                .execute_script("$width($data()) = $result();")
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "1024");
        })
    }

    #[test]
    fn test_obj() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            log::debug!("start");

            let mut cm = ClassManager::new();

            let mut ce = ClassExecutor::new(&mut cm);

            let rs = ce
                .execute_script(
                    r#"
1 = $sum();
2 = $pos();

<
    +(@{
        $left: $sum(),
        $right: $pos()
    }) := $sum();

    +(@{
        $left: $pos(),
        $right: 1
    }) := $pos();

    [
        @{
            $case: <#inner(@{$left: 101, $right: $pos()}) := $result();>,
            $then: <$() := $result();>
        },
        @{$case: <1 := $result();>}
    ] = $switch();
> = $loop();

$sum() := $result();
             "#,
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "5050");
        })
    }
}

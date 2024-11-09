use std::{
    collections::{BTreeSet, HashMap, HashSet},
    future::Future,
    pin::Pin,
};

use json::{array, object};
use util::{str_2_rs, Inc};

mod inner {
    use std::pin::Pin;

    use crate::{err, util::IncVal, AsClassManager, ClassExecutor, Fu};

    pub fn unwrap_value<'a, 'a1, 'f, CM: AsClassManager + Send + Sync>(
        ce: &'a ClassExecutor<'a, CM>,
        inc_val: &'a1 IncVal,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            match inc_val {
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

pub mod err;
pub mod util;

#[cfg(any(target_family = "wasm", feature = "no_send"))]
pub trait Fu: Future {}

#[cfg(any(target_family = "wasm", feature = "no_send"))]
impl<T: Future> Fu for T {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
pub trait Fu: Future + Send {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
impl<T: Future + Send> Fu for T {}

pub trait AsClassManager {
    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn get_source<'a, 'a1, 'a2, 'f>(
        &'a self,
        target: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;
}

#[allow(unused)]
pub struct Item {
    class: String,
    source: String,
    target: String,
}

pub struct ClassManager {
    unique_id: u64,
    class_mp: HashMap<u64, Item>,
    class_source_inx: HashMap<(String, String), BTreeSet<u64>>,
    target_class_inx: HashMap<(String, String), BTreeSet<u64>>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self {
            unique_id: 0,
            class_mp: HashMap::new(),
            class_source_inx: HashMap::new(),
            target_class_inx: HashMap::new(),
        }
    }
}

impl AsClassManager for ClassManager {
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
            if let Some(set) = self
                .class_source_inx
                .remove(&(class.to_string(), source.to_string()))
            {
                for id in set {
                    if let Some(item_class) = self.class_mp.remove(&id) {
                        self.target_class_inx
                            .remove(&(item_class.target, class.to_string()));
                    }
                }
            }

            Ok(())
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
            let mut id = self.unique_id;

            self.unique_id += target_v.len() as u64;

            for target in &target_v {
                self.class_mp.insert(
                    id,
                    Item {
                        class: class.to_string(),
                        source: source.to_string(),
                        target: target.clone(),
                    },
                );

                let class_pair_k = (class.to_string(), source.to_string());

                if let Some(set) = self.class_source_inx.get_mut(&class_pair_k) {
                    set.insert(id);
                } else {
                    let mut set = BTreeSet::new();

                    set.insert(id);

                    self.class_source_inx.insert(class_pair_k, set);
                }

                let target_class_k = (target.clone(), class.to_string());

                if let Some(set) = self.target_class_inx.get_mut(&target_class_k) {
                    set.insert(id);
                } else {
                    let mut set = BTreeSet::new();

                    set.insert(id);

                    self.target_class_inx.insert(target_class_k, set);
                }

                id += 1;
            }

            Ok(())
        })
    }

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
            let class_source_k = (class.to_string(), source.to_string());

            match self.class_source_inx.get(&class_source_k) {
                Some(set) => Ok(set
                    .iter()
                    .map(|id| self.class_mp.get(id).unwrap().target.clone())
                    .collect()),
                None => Ok(vec![]),
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
            let target_class_k = (target.to_string(), class.to_string());

            match self.target_class_inx.get(&target_class_k) {
                Some(set) => Ok(set
                    .iter()
                    .map(|id| self.class_mp.get(id).unwrap().target.clone())
                    .collect()),
                None => Ok(vec![]),
            }
        })
    }
}

pub struct ClassExecutor<'cm, CM: AsClassManager> {
    global_cm: &'cm mut CM,
    temp_cm: ClassManager,
}

impl<'cm, CM: AsClassManager + Send + Sync> ClassExecutor<'cm, CM> {
    pub fn execute<'a, 'a1, 'f>(
        &'a mut self,
        inc_v: &'a1 [Inc],
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
                    util::Opt::Append => {
                        for class in &class_v {
                            for source in &source_v {
                                self.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                    util::Opt::Set => {
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
            let inc_v = util::inc_v_from_str(script)?;

            self.execute(&inc_v).await
        })
    }
}

impl<'cm, CM: AsClassManager + Send + Sync> ClassExecutor<'cm, CM> {
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
            let mut rj = array![];

            for source in source_v {
                let mut item = object! {};

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

impl<'cm, CM: AsClassManager + Send + Sync> AsClassManager for ClassExecutor<'cm, CM> {
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
                        let class_v = self.get("$class", source).await?;
                        let source_v = self.get("$source", source).await?;

                        let rj = self.dump_json(&class_v, &source_v).await?;

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
                self.temp_cm.append(class, source, target_v).await
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
                    "test = test[test];
                    test[test] = $result[];",
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
                    "1 = $left[test];
                    1 = $right[test];
                    +[test] = $result[];",
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
                &object! {
                    "$width": 1024,
                    "$height": 1024
                },
            )
            .await
            .unwrap();

            let rs = ce
                .execute_script("$width[$data[]] = $result[];")
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "1024");
        })
    }
}

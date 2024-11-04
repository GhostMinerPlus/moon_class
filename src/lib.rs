pub mod err;
pub mod util;

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
};

use util::{Inc, IncVal};

#[cfg(target_family = "wasm")]
pub trait Fu: Future {}

#[cfg(target_family = "wasm")]
impl<T: Future> Fu for T {}

#[cfg(not(target_family = "wasm"))]
pub trait Fu: Future + Send {}

#[cfg(not(target_family = "wasm"))]
impl<T: Future + Send> Fu for T {}

pub trait AsClassManager: Send + Sync {
    fn execute<'a, 'a1, 'f>(
        &'a mut self,
        inc_v: &'a1 [Inc],
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let mut rs = vec![];

            for inc in inc_v {
                let class_v = self.unwrap_value(inc.class()).await?;
                let source_v = self.unwrap_value(inc.source()).await?;
                let target_v = self.unwrap_value(inc.target()).await?;

                for class in &class_v {
                    for source in &source_v {
                        match class.as_str() {
                            "$result" => {
                                rs = target_v.clone();
                            }
                            "$clear" => {
                                let addr = IncVal::from_str(source)?;

                                let (class, source) = addr.as_addr().unwrap();

                                self.clear(class.as_value().unwrap(), source.as_value().unwrap())
                                    .await?;
                            }
                            "$new" => {
                                let addr = IncVal::from_str(source)?;

                                let (class, source) = addr.as_addr().unwrap();

                                self.append(
                                    class.as_value().unwrap(),
                                    source.as_value().unwrap(),
                                    target_v.clone(),
                                )
                                .await?;
                            }
                            _ => {
                                self.clear(class, source).await?;
                                self.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                }
            }

            Ok(rs)
        })
    }

    fn unwrap_value<'a, 'a1, 'f>(
        &'a self,
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
                    let class_v = self.unwrap_value(class).await?;
                    let source_v = self.unwrap_value(source).await?;
                    let mut rs = vec![];

                    for class in &class_v {
                        for source in &source_v {
                            let target_v = self.get(class, source).await?;
                            rs.extend(target_v);
                        }
                    }

                    Ok(rs)
                }
            }
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
    class_source_inx: HashMap<(String, String), HashSet<u64>>,
    target_source_inx: HashMap<(String, String), HashSet<u64>>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self {
            unique_id: 0,
            class_mp: HashMap::new(),
            class_source_inx: HashMap::new(),
            target_source_inx: HashMap::new(),
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
                        self.target_source_inx
                            .remove(&(item_class.target, source.to_string()));
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
                    let mut set = HashSet::new();

                    set.insert(id);

                    self.class_source_inx.insert(class_pair_k, set);
                }

                let item_class_k = (target.clone(), class.to_string());

                if let Some(set) = self.target_source_inx.get_mut(&item_class_k) {
                    set.insert(id);
                } else {
                    let mut set = HashSet::new();

                    set.insert(id);

                    self.target_source_inx.insert(item_class_k, set);
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
            let class_pair_k = (class.to_string(), source.to_string());

            match self.class_source_inx.get(&class_pair_k) {
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

impl<'cm, CM: AsClassManager> ClassExecutor<'cm, CM> {
    pub fn new(global: &'cm mut CM) -> Self {
        Self {
            global_cm: global,
            temp_cm: ClassManager::new(),
        }
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
                .execute(
                    &util::inc_v_from_str(
                        "$new['test[test]'] = test;
                        $result[] = test[test];",
                    )
                    .unwrap(),
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
                .execute(
                    &util::inc_v_from_str(
                        "$left[test] = 1;
                        $right[test] = 1;
                        $result[] = +[test];",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }
}

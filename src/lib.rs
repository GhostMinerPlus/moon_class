pub mod err;
pub mod util;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    future::Future,
    pin::Pin,
};

use error_stack::ResultExt;
use util::Class;

#[cfg(target_family = "wasm")]
pub trait Fu: Future {}

#[cfg(target_family = "wasm")]
impl<T: Future> Fu for T {}

#[cfg(not(target_family = "wasm"))]
pub trait Fu: Future + Send {}

#[cfg(not(target_family = "wasm"))]
impl<T: Future + Send> Fu for T {}

pub trait AsClassManager: Send + Sync {
    fn call<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            log::debug!("call {}", class.to_string());

            if class.is_tag() {
                return Ok(vec![class.name.clone()]);
            } else if class.is_final_or_tag() {
                self.final_call(class).await
            } else {
                let left_item_v = self.call(class.left_op.as_ref().unwrap()).await?;
                let right_item_v = self.call(class.right_op.as_ref().unwrap()).await?;

                // Let 'append' be faster.
                if left_item_v.len() == 1 && class.name == "append" {
                    let left_class = Class::from_str(&left_item_v[0]);
                    self.append(&left_class.name, &left_class.pair()?, right_item_v)
                        .await?;

                    return Ok(vec![String::new()]);
                }

                let mut rs = vec![];

                for left_item in &left_item_v {
                    for right_item in &right_item_v {
                        let s_class = Class::new(
                            &class.name,
                            Some(Class::new_with_name(&left_item)),
                            Some(Class::new_with_name(&right_item)),
                        );

                        rs.extend(self.call(&s_class).await?);
                    }
                }

                Ok(rs)
            }
        })
    }

    fn final_call<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            match class.name.as_str() {
                "+" => {
                    let res = class
                        .left_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)
                        .attach_printable_lazy(|| {
                            format!("{} not a number!", class.left_op.as_ref().unwrap().name)
                        })?
                        + class
                            .right_op
                            .as_ref()
                            .unwrap()
                            .name
                            .parse::<f64>()
                            .change_context(err::Error::RuntimeError)
                            .attach_printable_lazy(|| {
                                format!("{} not a number!", class.right_op.as_ref().unwrap().name)
                            })?;
                    Ok(vec![res.to_string()])
                }
                "-" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)
                    .attach_printable_lazy(|| {
                        format!("{} not a number!", class.left_op.as_ref().unwrap().name)
                    })?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)
                        .attach_printable_lazy(|| {
                            format!("{} not a number!", class.right_op.as_ref().unwrap().name)
                        })?)
                .to_string()]),
                "*" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)
                    .attach_printable_lazy(|| {
                        format!("{} not a number!", class.left_op.as_ref().unwrap().name)
                    })?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)
                        .attach_printable_lazy(|| {
                            format!("{} not a number!", class.right_op.as_ref().unwrap().name)
                        })?)
                .to_string()]),
                "/" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)
                    .attach_printable_lazy(|| {
                        format!("{} not a number!", class.left_op.as_ref().unwrap().name)
                    })?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)
                        .attach_printable_lazy(|| {
                            format!("{} not a number!", class.right_op.as_ref().unwrap().name)
                        })?)
                .to_string()]),
                "new" => Ok(vec![uuid::Uuid::new_v4().to_string()]),
                "none" => Ok(vec![]),
                "unwrap_or" => {
                    if !class.left_op.as_ref().unwrap().name.is_empty() {
                        Ok(vec![class.left_op.as_ref().unwrap().name.clone()])
                    } else {
                        Ok(vec![class.right_op.as_ref().unwrap().name.clone()])
                    }
                }
                "return" => Ok(vec![class.right_op.as_ref().unwrap().name.clone()]),
                "pair" => Ok(vec![class.pair()?]),
                "pair_of" => {
                    self.pair_of(
                        &class.left_op.as_ref().unwrap().name,
                        &class.right_op.as_ref().unwrap().name,
                    )
                    .await
                }
                "type" => Ok(vec![format!(
                    "{}<{}>",
                    class.left_op.as_ref().unwrap().name.clone(),
                    class.right_op.as_ref().unwrap().name.clone()
                )]),
                "left_of_pair" => {
                    let pair = &class.left_op.as_ref().unwrap().name;

                    let pos = util::find_pat_ignoring_string(",", pair)
                        .ok_or(err::Error::RuntimeError)
                        .attach_printable_lazy(|| format!("{} is not a pair", pair))?;

                    Ok(vec![util::escape_word(&pair[0..pos].trim())])
                }
                "right_of_pair" => {
                    let pair = &class.left_op.as_ref().unwrap().name;

                    let pos = util::find_pat_ignoring_string(",", pair)
                        .ok_or(err::Error::RuntimeError)
                        .attach_printable_lazy(|| format!("{} is not a pair", pair))?;

                    Ok(vec![util::escape_word(pair[pos + 1..].trim())])
                }
                "clear" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);

                    self.clear(&left_class.name, &left_class.pair()?).await?;

                    Ok(vec![String::new()])
                }
                "append" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);

                    self.append(
                        &left_class.name,
                        &left_class.pair()?,
                        vec![class.right_op.as_ref().unwrap().name.clone()],
                    )
                    .await?;

                    Ok(vec![String::new()])
                }
                "or" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);
                    let right_class = Class::from_str(&class.right_op.as_ref().unwrap().name);

                    let mut rs = self.call(&left_class).await?;

                    rs.extend(self.call(&right_class).await?);

                    Ok(rs)
                }
                "and" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);
                    let right_class = Class::from_str(&class.right_op.as_ref().unwrap().name);
                    let mut left_set = BTreeSet::new();
                    let mut rs = vec![];

                    let left_item_v = self.call(&left_class).await?;
                    let right_item_v = self.call(&right_class).await?;

                    left_set.extend(left_item_v);

                    for right_item in right_item_v {
                        if left_set.contains(&right_item) {
                            rs.push(right_item);
                        }
                    }

                    Ok(rs)
                }
                "minus" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);
                    let right_class = Class::from_str(&class.right_op.as_ref().unwrap().name);
                    let mut left_set = BTreeSet::new();

                    let left_item_v = self.call(&left_class).await?;
                    let right_item_v = self.call(&right_class).await?;

                    left_set.extend(left_item_v);

                    for right_item in &right_item_v {
                        left_set.remove(right_item);
                    }

                    Ok(left_set.into_iter().collect())
                }
                _ => self.get(&class.name, &class.pair()?).await,
            }
        })
    }

    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;

    fn pair_of<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        item: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;
}

pub struct ItemClass {
    item: String,
    pair: String,
}

pub struct ClassManager {
    unique_id: u64,
    class_mp: HashMap<u64, ItemClass>,
    class_pair_inx: HashMap<(String, String), HashSet<u64>>,
    item_class_inx: HashMap<(String, String), HashSet<u64>>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self {
            unique_id: 0,
            class_mp: HashMap::new(),
            class_pair_inx: HashMap::new(),
            item_class_inx: HashMap::new(),
        }
    }
}

impl AsClassManager for ClassManager {
    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if let Some(set) = self
                .class_pair_inx
                .remove(&(class.to_string(), pair.to_string()))
            {
                for id in set {
                    if let Some(item_class) = self.class_mp.remove(&id) {
                        self.item_class_inx
                            .remove(&(item_class.item, class.to_string()));
                    }
                }
            }

            Ok(())
        })
    }

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let mut id = self.unique_id;

            self.unique_id += item_v.len() as u64;

            for item in &item_v {
                self.class_mp.insert(
                    id,
                    ItemClass {
                        item: item.clone(),
                        pair: pair.to_string(),
                    },
                );

                let class_pair_k = (class.to_string(), pair.to_string());

                if let Some(set) = self.class_pair_inx.get_mut(&class_pair_k) {
                    set.insert(id);
                } else {
                    let mut set = HashSet::new();

                    set.insert(id);

                    self.class_pair_inx.insert(class_pair_k, set);
                }

                let item_class_k = (item.clone(), class.to_string());

                if let Some(set) = self.item_class_inx.get_mut(&item_class_k) {
                    set.insert(id);
                } else {
                    let mut set = HashSet::new();

                    set.insert(id);

                    self.item_class_inx.insert(item_class_k, set);
                }

                id += 1;
            }

            Ok(())
        })
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let class_pair_k = (class.to_string(), pair.to_string());

            match self.class_pair_inx.get(&class_pair_k) {
                Some(set) => Ok(set
                    .iter()
                    .map(|id| self.class_mp.get(id).unwrap().item.clone())
                    .collect()),
                None => Ok(vec![]),
            }
        })
    }

    fn pair_of<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        item: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let item_class_k = (item.to_string(), class.to_string());

            match self.item_class_inx.get(&item_class_k) {
                Some(set) => Ok(set
                    .iter()
                    .map(|id| self.class_mp.get(id).unwrap().pair.clone())
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
    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.clear(class, pair).await
            } else {
                self.global_cm.clear(class, pair).await
            }
        })
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.get(class, pair).await
            } else {
                self.global_cm.get(class, pair).await
            }
        })
    }

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.append(class, pair, item_v).await
            } else {
                self.global_cm.append(class, pair, item_v).await
            }
        })
    }

    fn pair_of<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        item: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('$') {
                self.temp_cm.pair_of(item, class).await
            } else {
                self.global_cm.pair_of(item, class).await
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
                .call(&Class::from_str(
                    "return<append<'test<test, test>', +<1, 1>>, test<test, test>>",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }

    #[test]
    fn test_none() {
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
                .call(&Class::from_str(
                    "return<
                        append<'test<test, test>', +<1, 1>>,
                        none<, >,
                        test<test, test>
                    >",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 0);
        })
    }

    #[test]
    fn test_new() {
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
                .call(&Class::from_str(
                    "return<
                        append<'test<test, test>', new<, >>,
                        test<test, test>
                    >",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
        })
    }

    #[test]
    fn test_left_of_pair() {
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
                .call(&Class::from_str("left_of_pair<'\\'left \\', right', >"))
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "left ");
        })
    }
}

pub mod err;
pub mod util;

use std::{
    collections::{HashMap, HashSet},
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

pub type Auth = Option<PermissionPair>;

#[derive(Clone)]
pub struct PermissionPair {
    pub writer: HashSet<String>,
    pub reader: HashSet<String>,
}

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
            log::debug!("{}", class.to_string());

            if class.is_tag() {
                return Ok(vec![class.name.clone()]);
            } else if class.is_final_or_tag() {
                self.final_call(class).await
            } else {
                let left_item_v = self.call(class.left_op.as_ref().unwrap()).await?;
                let right_item_v = self.call(class.right_op.as_ref().unwrap()).await?;
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
                "+" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)?
                    + class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)?)
                .to_string()]),
                "-" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)?)
                .to_string()]),
                "*" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)?)
                .to_string()]),
                "/" => Ok(vec![(class
                    .left_op
                    .as_ref()
                    .unwrap()
                    .name
                    .parse::<f64>()
                    .change_context(err::Error::RuntimeError)?
                    - class
                        .right_op
                        .as_ref()
                        .unwrap()
                        .name
                        .parse::<f64>()
                        .change_context(err::Error::RuntimeError)?)
                .to_string()]),
                "none" => Ok(vec![]),
                "unwrap_or" => {
                    if !class.left_op.as_ref().unwrap().name.is_empty() {
                        Ok(vec![class.left_op.as_ref().unwrap().name.clone()])
                    } else {
                        Ok(vec![class.right_op.as_ref().unwrap().name.clone()])
                    }
                }
                "right" => Ok(vec![class.right_op.as_ref().unwrap().name.clone()]),
                "pair" => Ok(vec![format!(
                    "{}, {}",
                    class.left_op.as_ref().unwrap().name.clone(),
                    class.right_op.as_ref().unwrap().name.clone()
                )]),
                "type" => Ok(vec![format!(
                    "{}<{}>",
                    class.left_op.as_ref().unwrap().name.clone(),
                    class.right_op.as_ref().unwrap().name.clone()
                )]),
                "clear" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);

                    self.clear(&left_class).await?;

                    Ok(vec![String::new()])
                }
                "append" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);

                    self.append(&left_class, class.right_op.as_ref().unwrap().name.clone())
                        .await?;

                    Ok(vec![String::new()])
                }
                "minus" => {
                    let left_class = Class::from_str(&class.left_op.as_ref().unwrap().name);

                    self.minus(&left_class, &class.right_op.as_ref().unwrap().name)
                        .await?;

                    Ok(vec![String::new()])
                }
                _ => self.get(class).await,
            }
        })
    }

    fn clear<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f;

    fn get<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f;

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: String,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f;

    fn minus<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f;
}

pub struct ClassManager {
    class_mp: HashMap<Class, HashSet<String>>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self {
            class_mp: HashMap::new(),
        }
    }
}

impl AsClassManager for ClassManager {
    fn clear<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            self.class_mp.remove(class);

            Ok(())
        })
    }

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: String,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            match self.class_mp.get_mut(class) {
                Some(set) => {
                    set.insert(item);
                }
                None => {
                    let mut set = HashSet::new();

                    set.insert(item);

                    self.class_mp.insert(class.clone(), set);
                }
            }

            Ok(())
        })
    }

    fn minus<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if let Some(set) = self.class_mp.get_mut(class) {
                set.remove(item);
            }

            Ok(())
        })
    }

    fn get<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            match self.class_mp.get(class) {
                Some(set) => Ok(set.iter().map(|item| item.clone()).collect()),
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
    fn clear<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            if class.name.starts_with('$') {
                self.temp_cm.clear(class).await
            } else {
                self.global_cm.clear(class).await
            }
        })
    }

    fn get<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            if class.name.starts_with('$') {
                self.temp_cm.get(class).await
            } else {
                self.global_cm.get(class).await
            }
        })
    }

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: String,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            if class.name.starts_with('$') {
                self.temp_cm.append(class, item).await
            } else {
                self.global_cm.append(class, item).await
            }
        })
    }

    fn minus<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.name.starts_with('$') {
                self.temp_cm.minus(class, item).await
            } else {
                self.global_cm.minus(class, item).await
            }
        })
    }

    fn call<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        self.global_cm.call(class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_right() {
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
                    "right<append<'test\\<test\\, test\\>', +<1, 1>>, test<test, test>>",
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
                    "right<append<type<test, pair<test, test>>, none<, >, +<1, 1>>, test<test, test>>",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 0);
        })
    }
}

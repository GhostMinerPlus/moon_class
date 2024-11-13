use std::{
    collections::{BTreeSet, HashMap},
    future::Future,
    pin::Pin,
};

pub mod err;
pub mod util;

#[cfg(any(target_family = "wasm", feature = "no_send"))]
pub trait AsSendSyncOption {}

#[cfg(any(target_family = "wasm", feature = "no_send"))]
impl<T> AsSendSyncOption for T {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
pub trait AsSendSyncOption: Send + Sync {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
impl<T: Send + Sync> AsSendSyncOption for T {}

#[cfg(any(target_family = "wasm", feature = "no_send"))]
pub trait AsSendOption {}

#[cfg(any(target_family = "wasm", feature = "no_send"))]
impl<T> AsSendOption for T {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
pub trait AsSendOption: Send {}

#[cfg(all(not(target_family = "wasm"), not(feature = "no_send")))]
impl<T: Send> AsSendOption for T {}

pub trait Fu: Future + AsSendOption {}

impl<T: Future + AsSendOption> Fu for T {}

pub trait AsClassManager: AsSendSyncOption {
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
    source_inx: HashMap<String, BTreeSet<u64>>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self {
            unique_id: 0,
            class_mp: HashMap::new(),
            class_source_inx: HashMap::new(),
            target_class_inx: HashMap::new(),
            source_inx: HashMap::new(),
        }
    }

    pub fn dump(&self, source: &str) -> json::JsonValue {
        if let Some(set) = self.source_inx.get(source) {
            let mut obj = json::object! {};

            for id in set {
                let item = self.class_mp.get(id).unwrap();

                log::debug!("dump: {source}->{}: {}", item.class, item.target);

                if let json::JsonValue::Array(vec) = &mut obj[&item.class] {
                    vec.push(self.dump(&item.target));
                } else {
                    obj[&item.class] = json::array![self.dump(&item.target)];
                }
            }

            return obj;
        }

        return json::JsonValue::String(source.to_string());
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
                        self.source_inx.remove(&item_class.source);
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

                if let Some(set) = self.source_inx.get_mut(source) {
                    set.insert(id);
                } else {
                    let mut set = BTreeSet::new();

                    set.insert(id);

                    self.source_inx.insert(source.to_string(), set);
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

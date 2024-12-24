use std::{
    collections::{BTreeSet, HashMap},
    pin::Pin,
};

mod bean;

pub mod def;
pub mod err;
pub mod executor;
pub mod util;

pub struct ClassManager {
    unique_id: u64,
    class_mp: HashMap<u64, bean::Item>,
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

            obj
        } else {
            json::JsonValue::String(source.to_string())
        }
    }

    pub fn get_source(&self, target: &str, class: &str) -> Option<Vec<String>> {
        let target_class_k = (target.to_string(), class.to_string());

        match self.target_class_inx.get(&target_class_k) {
            Some(set) => Some(
                set.iter()
                    .map(|id| self.class_mp.get(id).unwrap().source.clone())
                    .collect(),
            ),
            None => None,
        }
    }

    pub fn get_target(&self, class: &str, source: &str) -> Option<Vec<String>> {
        let class_source_k = (class.to_string(), source.to_string());

        if let Some(set) = self.class_source_inx.get(&class_source_k) {
            Some(
                set.iter()
                    .map(|id| self.class_mp.get(id).unwrap().target.clone())
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl def::AsClassManager for ClassManager {
    fn remove<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
    ) -> Pin<Box<dyn def::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let mut target_set = BTreeSet::new();

            target_set.extend(target_v);

            let class_source_k = (class.to_string(), source.to_string());

            if let Some(set) = self.class_source_inx.get_mut(&class_source_k) {
                let id_v = set
                    .iter()
                    .filter(|id| {
                        if let Some(item_class) = self.class_mp.get(&id) {
                            if target_set.contains(&item_class.target) {
                                return true;
                            }
                        }

                        false
                    })
                    .map(|id| *id)
                    .collect::<Vec<u64>>();

                for id in &id_v {
                    set.remove(id);

                    if let Some(item_class) = self.class_mp.remove(&id) {
                        if let Some(set) = self
                            .target_class_inx
                            .get_mut(&(item_class.target, class.to_string()))
                        {
                            set.remove(&id);
                        }
                        if let Some(set) = self.source_inx.get_mut(&item_class.source) {
                            set.remove(&id);
                        }
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
    ) -> Pin<Box<dyn def::Fu<Output = err::Result<()>> + 'f>>
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
                    bean::Item {
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
    ) -> Pin<Box<dyn def::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            match class {
                "#source" => {
                    let data = json::parse(source).unwrap();

                    Ok(self
                        .get_source(
                            data["$target"][0].as_str().unwrap(),
                            data["$class"][0].as_str().unwrap(),
                        )
                        .unwrap_or_default())
                }
                _ => Ok(self.get_target(class, source).unwrap_or_default()),
            }
        })
    }
}

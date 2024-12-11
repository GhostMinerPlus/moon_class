use std::{collections::HashSet, fs, path::Path, pin::Pin};

use inc::inc_v_from_str;

use crate::{err, AsClassManager, AsSendSyncOption, ClassManager, Fu};

use super::{rs_2_str, str_2_rs};

mod string;
mod value_extractor;
mod inner {
    use std::pin::Pin;

    use crate::{err, util::executor::inc, AsClassManager, Fu};

    use super::*;

    pub fn unwrap_value<'a, 'a1, 'f, CM>(
        ce: &'a mut CM,
        inc_val: &'a1 inc::IncVal,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        CM: AsClassManager,
    {
        Box::pin(async move {
            match inc_val {
                inc::IncVal::Object(s) => value_extractor::object(ce, s).await,
                inc::IncVal::Script(v) => value_extractor::script(ce, v).await,
                inc::IncVal::Value(v) => Ok(vec![v.clone()]),
                inc::IncVal::Addr((class, source)) => {
                    let class_v = unwrap_value(ce, class).await?;
                    let source_v = unwrap_value(ce, source).await?;
                    let mut rs = vec![];

                    for class in &class_v {
                        for source in &source_v {
                            rs.extend(ce.get(class, &source).await?);
                        }
                    }

                    Ok(rs)
                }
            }
        })
    }

    pub fn execute<'a, 'a1, 'f, CM>(
        ce: &'a mut CM,
        inc_v: &'a1 [inc::Inc],
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        CM: AsClassManager,
    {
        Box::pin(async move {
            for inc in inc_v {
                log::debug!("execute: {inc}");

                let class_v = unwrap_value(ce, inc.class()).await?;
                let source_v = unwrap_value(ce, inc.source()).await?;
                let target_v = unwrap_value(ce, inc.target()).await?;

                match inc.operator() {
                    inc::Opt::Append => {
                        for class in &class_v {
                            for source in &source_v {
                                ce.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                    inc::Opt::Remove => {
                        for class in &class_v {
                            for source in &source_v {
                                ce.remove(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                    inc::Opt::Set => {
                        for class in &class_v {
                            for source in &source_v {
                                ce.remove(class, source, ce.get(class, source).await?)
                                    .await?;

                                ce.append(class, source, target_v.clone()).await?;
                            }
                        }
                    }
                }
            }

            ce.get("$result", "").await
        })
    }

    pub fn execute_script<'a, 'a1, 'f, CM>(
        ce: &'a mut CM,
        script: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        CM: AsClassManager,
    {
        Box::pin(async move {
            let inc_v = inc::inc_v_from_str(script)?;

            log::debug!("{:?}", inc_v);

            execute(ce, &inc_v).await
        })
    }
}

pub mod inc;

pub trait ClassManagerHolder {
    type CM: AsClassManager;

    fn temp_ref(&self) -> &ClassManager;

    fn temp_mut(&mut self) -> &mut ClassManager;

    fn global_ref(&self) -> &Self::CM;

    fn global_mut(&mut self) -> Option<&mut Self::CM>;

    fn path_mut(&mut self) -> &mut String;
}

pub struct ClassExecutor<'cm, CM> {
    global_cm: &'cm mut CM,
    temp_cm: ClassManager,
    path: String,
}

impl<'cm, CM> ClassExecutor<'cm, CM> {
    pub fn new(global: &'cm mut CM) -> Self {
        Self {
            global_cm: global,
            temp_cm: ClassManager::new(),
            path: ".".to_string(),
        }
    }
}

impl<'cm, CM: AsClassManager> ClassExecutor<'cm, CM> {
    pub fn execute_script<'a, 'a1, 'f>(
        &'a mut self,
        script: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        inner::execute_script(self, script)
    }

    pub fn temp(self) -> ClassManager {
        self.temp_cm
    }
}

impl<'cm, AsCM: AsClassManager> ClassManagerHolder for ClassExecutor<'cm, AsCM> {
    type CM = AsCM;

    fn temp_ref(&self) -> &ClassManager {
        &self.temp_cm
    }

    fn temp_mut(&mut self) -> &mut ClassManager {
        &mut self.temp_cm
    }

    fn global_ref(&self) -> &Self::CM {
        self.global_cm
    }

    fn global_mut(&mut self) -> Option<&mut Self::CM> {
        Some(self.global_cm)
    }

    fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }
}

impl<'cm, T, AsCM> AsClassManager for T
where
    AsCM: AsClassManager,
    T: ClassManagerHolder<CM = AsCM> + AsSendSyncOption,
{
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
                self.temp_ref().get(class, source).await
            } else if class.starts_with('#') {
                match class {
                    "#fract" => Ok(vec![source.parse::<f64>().unwrap().fract().to_string()]),
                    "#dump" => {
                        let rj = self.temp_ref().dump(source);

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
                    "#slice" => {
                        let source_v = self.get("$source", source).await?;
                        let from_v = self.get("$from", source).await?;
                        let to_v = self.get("$to", source).await?;

                        let from = match from_v.first() {
                            Some(s) => s.parse().unwrap(),
                            None => 0,
                        };
                        let to = match to_v.first() {
                            Some(s) => s.parse().unwrap(),
                            None => source_v.len(),
                        };

                        Ok(source_v[from..to].iter().map(|s| s.clone()).collect())
                    }
                    "#index" => {
                        let source_v = self.get("$source", source).await?;
                        let index_v = self.get("$index", source).await?;

                        let index = index_v.first().unwrap().parse::<usize>().unwrap();

                        Ok(match source_v.get(index) {
                            Some(rs) => vec![rs.clone()],
                            None => vec![],
                        })
                    }
                    "#count" => {
                        let source_v = self.get("$source", source).await?;

                        Ok(vec![source_v.len().to_string()])
                    }
                    "#not" => {
                        let source_v = self.get("$source", source).await?;

                        let mut rs = vec![];

                        if source_v.is_empty() {
                            rs.push("1".to_string());
                        }

                        Ok(rs)
                    }
                    "#source" => {
                        let class_v = self.get("$class", source).await?;

                        if class_v[0].starts_with('$') {
                            let target_v = self.get("$target", source).await?;
                            let class_v = self.get("$class", source).await?;

                            Ok(self
                                .temp_ref()
                                .get_source(&target_v[0], &class_v[0])
                                .unwrap_or_default())
                        } else {
                            let data = self.temp_ref().dump(source);

                            self.global_ref().get(class, &data.to_string()).await
                        }
                    }
                    "#map" => {
                        todo!()
                    }
                    _ => {
                        let script_v = self.get("onget", class).await?;

                        if !script_v.is_empty() {
                            let mut ce = ReadOnlyClassExecutor::new(self.global_ref());

                            ce.append("$source", "", vec![source.to_string()]).await?;

                            ce.execute_script(&rs_2_str(&script_v)).await
                        } else {
                            self.global_ref().get(class, source).await
                        }
                    }
                }
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
                    "%" => {
                        let left_v = self.get("$left", source).await?;
                        let right_v = self.get("$right", source).await?;

                        let sz = left_v.len();

                        let mut rs = vec![];

                        for i in 0..sz {
                            let left = left_v[i].parse::<i32>().unwrap();
                            let right = right_v[i].parse::<i32>().unwrap();

                            rs.push((left % right).to_string());
                        }

                        Ok(rs)
                    }
                    _ => self.global_ref().get(class, source).await,
                }
            }
        })
    }

    fn remove<'a, 'a1, 'a2, 'f>(
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
                self.temp_mut().remove(class, source, target_v).await
            } else if class.starts_with('#') {
                let script_v = self.get("onremove", class).await?;

                if !script_v.is_empty() {
                    let mut ce = ClassExecutor::new(self.global_mut().unwrap());

                    ce.append("$source", "", vec![source.to_string()]).await?;
                    ce.append("$target", "", target_v).await?;

                    ce.execute_script(&rs_2_str(&script_v)).await?;

                    Ok(())
                } else {
                    self.global_mut()
                        .unwrap()
                        .remove(class, source, target_v)
                        .await
                }
            } else {
                self.global_mut()
                    .unwrap()
                    .remove(class, source, target_v)
                    .await
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
                self.temp_mut().append(class, source, target_v).await
            } else if class.starts_with('#') {
                match class {
                    "#switch" => {
                        for target in &target_v {
                            let case_v = self.get("$case", target).await?;

                            if !inner::execute_script(self, &rs_2_str(&case_v))
                                .await?
                                .is_empty()
                            {
                                let then_v = self.get("$then", target).await?;

                                inner::execute_script(self, &rs_2_str(&then_v)).await?;

                                break;
                            }
                        }

                        Ok(())
                    }
                    "#loop" => {
                        let inc_v = inc_v_from_str(&rs_2_str(&target_v))?;

                        while !inner::execute(self, &inc_v).await?.is_empty() {}

                        Ok(())
                    }
                    "#load" => {
                        let source_v = self.get("$source", source).await?;
                        let class_v = self.get("$class", source).await?;

                        let target_v =
                            inner::unwrap_value(self, &inc::IncVal::Object(target_v[0].clone()))
                                .await?;

                        self.append(&class_v[0], &source_v[0], target_v).await
                    }
                    "#map" => {
                        log::debug!("#map: target_v = {target_v:?}");
                        let source_v = self.get("$source", source).await?;
                        let class_v = self.get("$class", source).await?;
                        let item_v = self.get("$source", &target_v[0]).await?;
                        let mapper_v = self.get("$mapper", &target_v[0]).await?;

                        let script = rs_2_str(&mapper_v);

                        log::debug!("#map: script = {mapper_v:?}");
                        let inc_v = inc_v_from_str(&script)?;

                        let mut rs = Vec::with_capacity(item_v.len());
                        let mut index = 0;

                        for item in item_v {
                            self.append("$item", "", vec![item.clone()]).await?;
                            self.append("$index", "", vec![index.to_string()]).await?;

                            rs.push(inner::execute(self, &inc_v).await?.join("\n"));

                            self.remove("$item", "", vec![item]).await?;
                            self.remove("$index", "", vec![index.to_string()]).await?;

                            index += 1;
                        }

                        self.append(&class_v[0], &source_v[0], rs).await
                    }
                    "#include" => {
                        for target in &target_v {
                            let root = self.path_mut().clone();
                            let file_path = format!("{root}/{target}");

                            println!("#include: file_path = {file_path}");

                            let file = Path::new(&file_path);

                            let dir = file
                                .parent()
                                .as_ref()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();
                            let script = fs::read_to_string(file).unwrap();

                            *self.path_mut() = dir;
                            inner::execute_script(self, &script).await?;

                            *self.path_mut() = root;
                        }

                        Ok(())
                    }
                    _ => {
                        let script_v = self.get("onappend", class).await?;

                        if !script_v.is_empty() {
                            let mut ce = ClassExecutor::new(self.global_mut().unwrap());

                            ce.append("$source", "", vec![source.to_string()]).await?;
                            ce.append("$target", "", target_v).await?;

                            ce.execute_script(&rs_2_str(&script_v)).await?;

                            Ok(())
                        } else {
                            self.global_mut()
                                .unwrap()
                                .append(class, source, target_v)
                                .await
                        }
                    }
                }
            } else {
                self.global_mut()
                    .unwrap()
                    .append(class, source, target_v)
                    .await
            }
        })
    }
}

pub struct ReadOnlyClassExecutor<'cm, CM> {
    global_cm: &'cm CM,
    temp_cm: ClassManager,
    path: String,
}

impl<'cm, CM> ReadOnlyClassExecutor<'cm, CM> {
    pub fn new(global: &'cm CM) -> Self {
        Self {
            global_cm: global,
            temp_cm: ClassManager::new(),
            path: ".".to_string(),
        }
    }

    pub fn new_with_temp(global: &'cm CM, temp_cm: ClassManager) -> Self {
        Self {
            global_cm: global,
            temp_cm,
            path: ".".to_string(),
        }
    }
}

impl<'cm, AsCM: AsClassManager> ClassManagerHolder for ReadOnlyClassExecutor<'cm, AsCM> {
    type CM = AsCM;

    fn temp_ref(&self) -> &ClassManager {
        &self.temp_cm
    }

    fn temp_mut(&mut self) -> &mut ClassManager {
        &mut self.temp_cm
    }

    fn global_ref(&self) -> &Self::CM {
        self.global_cm
    }

    fn global_mut(&mut self) -> Option<&mut Self::CM> {
        None
    }

    fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }
}

impl<'cm, CM: AsClassManager> ReadOnlyClassExecutor<'cm, CM> {
    pub fn execute_script<'a, 'a1, 'f>(
        &'a mut self,
        script: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        inner::execute_script(self, script)
    }

    pub fn temp(self) -> ClassManager {
        self.temp_cm
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
    ] = #switch();
> = #loop();

$sum() := $result();
             "#,
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "5050");
        })
    }

    #[test]
    fn test_template() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut cm = ClassManager::new();

            let mut ce = ClassExecutor::new(&mut cm);

            let rs = ce
                .execute_script(
                    r#"
value := $value();
<${$value()}> := $result();
            "#,
                )
                .await
                .unwrap();

            assert_eq!(rs[0], "value")
        });
    }
}

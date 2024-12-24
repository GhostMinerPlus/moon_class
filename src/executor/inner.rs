use std::pin::Pin;

use crate::{
    def::{AsClassManager, AsSetable, Fu},
    err,
};

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
                            ce.set(class, source, target_v.clone()).await?;
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

use std::{pin::Pin, sync::Arc};

use tokio::sync::Mutex;

use crate::{def::{AsClassManager, Fu}, ClassManager};

pub trait AsClassManagerHolder {
    type CM: AsClassManager;

    fn temp(&self) -> Arc<Mutex<ClassManager>>;

    fn global_ref(&self) -> &Self::CM;

    fn global_mut(&mut self) -> Option<&mut Self::CM>;

    fn path_mut(&mut self) -> &mut String;

    fn dump<'a, 'a1, 'f>(
        &'a self,
        source: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = json::JsonValue> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        let temp = self.temp().clone();
        Box::pin(async move {
            let temp = temp.lock().await;

            temp.dump(source)
        })
    }
}

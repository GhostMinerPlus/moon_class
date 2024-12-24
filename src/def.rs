use std::{future::Future, pin::Pin};

use crate::err;


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

    fn remove<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
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

pub trait AsSetable {
    fn set<'a, 'a1, 'a2, 'f>(
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

impl<T: AsClassManager> AsSetable for T {
    fn set<'a, 'a1, 'a2, 'f>(
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
            self.remove(class, source, self.get(class, source).await?)
                .await?;

            self.append(class, source, target_v.clone()).await
        })
    }
}

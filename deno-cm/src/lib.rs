use std::{cell::RefCell, rc::Rc};

use deno_core::{serde_v8, v8, Extension, JsRuntime, OpDecl, PollEventLoopOptions, RuntimeOptions};
use inner::JsClassManager;
use moon_class::AsClassManager;

mod inner {
    use std::{cell::RefCell, rc::Rc};

    use deno_core::{op2, OpState, Resource};
    use moon_class::AsClassManager;

    pub struct JsClassManager {
        cm: Rc<RefCell<dyn AsClassManager>>,
    }

    impl JsClassManager {
        pub fn new(cm: Rc<RefCell<dyn AsClassManager>>) -> Self {
            Self { cm }
        }

        pub fn cm(&self) -> Rc<RefCell<dyn AsClassManager>> {
            self.cm.clone()
        }
    }

    impl Resource for JsClassManager {}

    /// An op for summing an array of numbers. The op-layer automatically
    /// deserializes inputs and serializes the returned Result & value.
    #[op2(async)]
    pub async fn cm_append(
        op_state_cell: Rc<RefCell<OpState>>,
        #[string] class: String,
        #[string] source: String,
        #[serde] target_v: Vec<String>,
    ) -> Result<(), deno_core::error::AnyError> {
        log::debug!("cm_append calling");

        let cm_cell = op_state_cell
            .borrow()
            .resource_table
            .get::<JsClassManager>(0)
            .unwrap()
            .cm();

        cm_cell
            .borrow_mut()
            .append(&class, &source, target_v)
            .await
            .unwrap();

        Ok(())
    }

    /// An op for summing an array of numbers. The op-layer automatically
    /// deserializes inputs and serializes the returned Result & value.
    #[op2(async)]
    pub async fn cm_clear(
        op_state_cell: Rc<RefCell<OpState>>,
        #[string] class: String,
        #[string] source: String,
    ) -> Result<(), deno_core::error::AnyError> {
        let cm_cell = op_state_cell
            .borrow()
            .resource_table
            .get::<JsClassManager>(0)
            .unwrap()
            .cm();

        cm_cell.borrow_mut().clear(&class, &source).await.unwrap();

        Ok(())
    }

    #[op2(async)]
    #[serde]
    pub async fn cm_get(
        op_state_cell: Rc<RefCell<OpState>>,
        #[string] class: String,
        #[string] source: String,
    ) -> Result<Vec<String>, deno_core::error::AnyError> {
        let cm_cell = op_state_cell
            .borrow()
            .resource_table
            .get::<JsClassManager>(0)
            .unwrap()
            .cm();

        let rs = cm_cell.borrow().get(&class, &source).await.unwrap();

        Ok(rs)
    }

    #[op2(async)]
    #[serde]
    pub async fn cm_get_source(
        op_state_cell: Rc<RefCell<OpState>>,
        #[string] target: String,
        #[string] class: String,
    ) -> Result<Vec<String>, deno_core::error::AnyError> {
        let cm_cell = op_state_cell
            .borrow()
            .resource_table
            .get::<JsClassManager>(0)
            .unwrap()
            .cm();

        let rs = cm_cell.borrow().get_source(&target, &class).await.unwrap();

        Ok(rs)
    }
}

pub mod err;

pub struct CmRuntime {
    js_runtime: JsRuntime,
}

impl CmRuntime {
    pub fn new(cm: Rc<RefCell<dyn AsClassManager>>) -> Self {
        // Build a deno_core::Extension providing custom ops
        const APPEND_DECL: OpDecl = inner::cm_append();
        const CLEAR_DECL: OpDecl = inner::cm_clear();

        let ext = Extension {
            name: "cm_ext",
            ops: std::borrow::Cow::Borrowed(&[APPEND_DECL, CLEAR_DECL]),
            op_state_fn: Some(Box::new(|op_state| {
                let id = op_state.resource_table.add(inner::JsClassManager::new(cm));

                log::debug!("op_state_init: {id}");
            })),
            ..Default::default()
        };

        // Initialize a runtime instance
        let js_runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![ext],
            ..Default::default()
        });

        Self { js_runtime }
    }

    pub async fn execute_script(&mut self, script: String) -> err::Result<serde_json::Value> {
        let promise = self.js_runtime.execute_script("", script).map_err(|e| {
            log::error!("{e}");

            err::Error::RuntimeError
        })?;

        let fu = self.js_runtime.resolve(promise);

        let global = self
            .js_runtime
            .with_event_loop_promise(fu, PollEventLoopOptions::default())
            .await
            .map_err(|e| {
                log::error!("{e}");

                err::Error::RuntimeError
            })?;

        let scope = &mut self.js_runtime.handle_scope();
        let local = v8::Local::new(scope, global);
        // Deserialize a `v8` object into a Rust type using `serde_v8`,
        // in this case deserialize to a JSON `Value`.
        let rs = serde_v8::from_v8::<serde_json::Value>(scope, local).map_err(|e| {
            log::error!("{e}");

            err::Error::RuntimeError
        })?;

        Ok(rs)
    }

    pub fn cm_cell(&mut self) -> Rc<RefCell<dyn AsClassManager>> {
        let js_cm = self
            .js_runtime
            .op_state()
            .borrow()
            .resource_table
            .get::<JsClassManager>(0)
            .unwrap();

        js_cm.cm()
    }
}

#[cfg(test)]
mod tests {
    use moon_class::ClassManager;

    use super::*;

    #[test]
    fn test() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut runtime = CmRuntime::new(Rc::new(RefCell::new(ClassManager::new())));

            let rs = runtime
                .execute_script(
                    r#"
Deno.core.ops.cm_append("test", "test", ["test"])
"#
                    .to_string(),
                )
                .await
                .unwrap();

            println!("execute_script: {rs}");

            {
                let cm_cell = runtime.cm_cell();

                let rs = cm_cell.borrow().get("test", "test").await.unwrap();

                println!("rs = {rs:?}");
            }
        })
    }
}

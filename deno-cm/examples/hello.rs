// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
//!  This example shows you how to define ops in Rust and then call them from
//!  JavaScript.

use deno_core::*;
use moon_class::{AsClassManager, ClassManager};

static mut CM: Option<Box<dyn AsClassManager>> = None;

/// An op for summing an array of numbers. The op-layer automatically
/// deserializes inputs and serializes the returned Result & value.
#[op2(async)]
async fn op_sum(#[serde] nums: Vec<f64>) -> Result<f64, deno_core::error::AnyError> {
    unsafe {
        let cm = CM.as_mut().unwrap();

        let _ = cm
            .append("op_sum", "", nums.iter().map(|n| n.to_string()).collect())
            .await;
    }

    // Sum inputs
    let sum = nums.iter().fold(0.0, |a, v| a + v);
    // return as a Result<f64, AnyError>
    Ok(sum)
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        unsafe {
            CM = Some(Box::new(ClassManager::new()));
        };

        // Build a deno_core::Extension providing custom ops
        const DECL: OpDecl = op_sum();
        let ext = Extension {
            name: "my_ext",
            ops: std::borrow::Cow::Borrowed(&[DECL]),
            ..Default::default()
        };

        // Initialize a runtime instance
        let mut runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![ext],
            ..Default::default()
        });

        // Now we see how to invoke the op we just defined. The runtime automatically
        // contains a Deno.core object with several functions for interacting with it.
        // You can find its definition in core.js.
        runtime
            .execute_script(
                "<usage>",
                r#"
// Print helper function, calling Deno.core.print()
function print(value) {
Deno.core.print(value.toString()+"\n");
}

async function main() {
const arr = [1, 2, 3];
print("The sum of");
print(arr);
print("is");
print(await Deno.core.ops.op_sum(arr));

// And incorrect usage
try {
  print(await Deno.core.ops.op_sum(0));
} catch(e) {
  print('Exception:');
  print(e);
}
}

main();
"#,
            )
            .unwrap();

        unsafe {
            let rs = CM.as_ref().unwrap().get("op_sum", "").await.unwrap();

            println!("rs = {rs:?}");
        }
    })
}

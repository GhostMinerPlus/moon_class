use moon_class::{util::executor::ClassExecutor, ClassManager};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut cm = ClassManager::new();

        let mut ce = ClassExecutor::new(&mut cm);

        let rs = ce
            .execute_script("<1 = $test();> = $test();<${\"<\"}${$test()}${\">\"}> = $result();")
            .await
            .unwrap();

        println!("rs = {}", rs[0]);
    })
}

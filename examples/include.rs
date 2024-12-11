use moon_class::{util::executor::ClassExecutor, ClassManager};

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu=warn,world_plugin=debug"),
    )
    .init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut cm = ClassManager::new();

        let mut ce = ClassExecutor::new(&mut cm);

        ce.execute_script("\"assets/class/main.class\" = #include();")
            .await
            .unwrap();
    })
}

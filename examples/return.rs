use moon_class::{util::executor::ClassExecutor, ClassManager};

fn main() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
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

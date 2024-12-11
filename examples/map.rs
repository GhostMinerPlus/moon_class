use moon_class::{util::executor::{ClassExecutor, ClassManagerHolder}, ClassManager};

fn main() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .is_test(true)
        .try_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut cm = ClassManager::new();

        let rj = {
            let mut ce = ClassExecutor::new(&mut cm);

            let rs = ce
                .execute_script(
                    r#"
[1, 2, 3, 4] = $univese();

{
    $source: $univese(),
    $mapper: <
        {
            $item: $item(),
            $index: $index()
        } := $result();
    >
} = #map({
    $source: $,
    $class: $temp
});

$temp($) := $result();
    "#,
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 4);

            println!("rs = {rs:?}");

            ce.temp_ref().dump(&rs[0])
        };

        println!("rj = {rj}");
    });
}

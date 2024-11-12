use error_stack::ResultExt;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Row, Sqlite};
use std::pin::Pin;

use moon_class::{err, AsClassManager};

const CLASS_INIT_SQL: &str = "-- class_t definition

CREATE TABLE class_t (
    id integer PRIMARY KEY,
    class varchar(500),
    source varchar(500),
    target varchar(500)
);

CREATE INDEX class_t_class_source ON class_t (class, source);
CREATE INDEX class_t_target_IDX ON class_t (target,class);";

pub struct SqliteClassManager {
    pool: Pool<Sqlite>,
}

impl SqliteClassManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn new_with_file(uri: &str) -> Self {
        let pool = sqlx::SqlitePool::connect_with(SqliteConnectOptions::new().filename(uri))
            .await
            .unwrap();
        Self { pool }
    }

    pub async fn init(&self) {
        sqlx::query(CLASS_INIT_SQL)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

impl AsClassManager for SqliteClassManager {
    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            sqlx::query(&format!("DELETE FROM class_t WHERE class=? AND source = ?"))
                .bind(class)
                .bind(source)
                .execute(&self.pool)
                .await
                .change_context(moon_class::err::Error::RuntimeError)?;

            Ok(())
        })
    }

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            for target in &target_v {
                sqlx::query(&format!(
                    "INSERT INTO class_t(class, source, target) VALUES (?, ?, ?)"
                ))
                .bind(class)
                .bind(source)
                .bind(target)
                .execute(&self.pool)
                .await
                .change_context(moon_class::err::Error::RuntimeError)?;
            }

            Ok(())
        })
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let rs = sqlx::query(&format!(
                "SELECT target FROM class_t WHERE class=? AND source = ? ORDER BY id"
            ))
            .bind(class)
            .bind(source)
            .fetch_all(&self.pool)
            .await
            .change_context(moon_class::err::Error::RuntimeError)?;

            let mut arr = vec![];

            for row in rs {
                arr.push(row.get(0));
            }

            Ok(arr)
        })
    }

    fn get_source<'a, 'a1, 'a2, 'f>(
        &'a self,
        target: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let rs = sqlx::query(&format!(
                "SELECT source FROM class_t WHERE target=? AND class=? ORDER BY id"
            ))
            .bind(target)
            .bind(class)
            .fetch_all(&self.pool)
            .await
            .change_context(moon_class::err::Error::RuntimeError)?;

            let mut arr = vec![];

            for row in rs {
                arr.push(row.get(0));
            }

            Ok(arr)
        })
    }
}

#[cfg(test)]
mod tests {
    use moon_class::{
        util::executor::ClassExecutor,
        ClassManager,
    };

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
                    "1 = $left[test];
                        1 = $right[test];
                        +[test] = $result[];",
                )
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }
}

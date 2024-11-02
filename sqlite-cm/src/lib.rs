use error_stack::ResultExt;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Row, Sqlite};
use std::pin::Pin;

use moon_class::{err, AsClassManager};

const CLASS_INIT_SQL: &str = "CREATE TABLE IF NOT EXISTS class_t (
    id integer PRIMARY KEY,
    item_name varchar(500),
    class_name varchar(500),
    pair varchar(500)
);
CREATE INDEX IF NOT EXISTS class_t_class_pair ON class_t (class_name, pair);
CREATE INDEX IF NOT EXISTS class_t_item_class ON class_t (item_name, class_name);";

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
        pair: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            sqlx::query(&format!(
                "DELETE FROM class_t WHERE class_name=? and pair = ?"
            ))
            .bind(class)
            .bind(pair)
            .execute(&self.pool)
            .await
            .change_context(moon_class::err::Error::RuntimeError)?;

            Ok(())
        })
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let rs = sqlx::query(&format!(
                "SELECT item_name FROM class_t WHERE class_name=? and pair = ?"
            ))
            .bind(class)
            .bind(pair)
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

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            for item in &item_v {
                sqlx::query(&format!(
                    "INSERT INTO class_t(item_name, class_name, pair) VALUES (?, ?, ?)"
                ))
                .bind(item)
                .bind(class)
                .bind(pair)
                .execute(&self.pool)
                .await
                .change_context(moon_class::err::Error::RuntimeError)?;
            }

            Ok(())
        })
    }

    fn pair_of<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        item: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            let rs = sqlx::query(&format!(
                "SELECT pair FROM class_t WHERE item_name=? and class_name=?"
            ))
            .bind(item)
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
    use moon_class::{util::Class, ClassExecutor};

    use super::*;

    #[test]
    fn test_right() {
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

            let mut cm = SqliteClassManager::new_with_file("test.db").await;

            let rs = ClassExecutor::new(&mut cm)
                .call(&Class::from_str(
                    "right<append<type<$test, pair<test, test>>, +<1, 1>>, $test<test, test>>",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }
}

use error_stack::ResultExt;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Row, Sqlite};
use std::pin::Pin;

use moon_class::{err, util::Class, AsClassManager};

const CLASS_INIT_SQL: &str = "CREATE TABLE IF NOT EXISTS class_t (
    id integer PRIMARY KEY,
    item_name varchar(500),
    class_name varchar(500),
    class_left varchar(500),
    class_right varchar(500)
);
CREATE INDEX IF NOT EXISTS class_t_class ON class_t (class_name, class_left, class_right);
CREATE INDEX IF NOT EXISTS class_t_item ON class_t (item_name);";

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
    fn clear<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            sqlx::query(&format!(
                "DELETE FROM class_t WHERE class_name=? and class_left=? and class_right=?"
            ))
            .bind(&class.name)
            .bind(&class.left_op.as_ref().unwrap().name)
            .bind(&class.right_op.as_ref().unwrap().name)
            .execute(&self.pool)
            .await
            .change_context(moon_class::err::Error::RuntimeError)?;

            Ok(())
        })
    }

    fn get<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 Class,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let rs = sqlx::query(&format!(
                "SELECT item_name FROM class_t WHERE class_name=? and class_left=? and class_right=?"
            ))
            .bind(&class.name)
            .bind(&class.left_op.as_ref().unwrap().name)
            .bind(&class.right_op.as_ref().unwrap().name)
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

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        class: &'a1 Class,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn moon_class::Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            for item in &item_v {
                sqlx::query(&format!(
                    "INSERT INTO class_t(item_name, class_name, class_left, class_right) VALUES (?, ?, ?, ?)"
                ))
                .bind(item)
                .bind(&class.name)
                .bind(&class.left_op.as_ref().unwrap().name)
                .bind(&class.right_op.as_ref().unwrap().name)
                .execute(&self.pool)
                .await
                .change_context(moon_class::err::Error::RuntimeError)?;
            }

            Ok(())
        })
    }

    fn minus<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 Class,
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
                    "DELETE FROM class_t WHERE item_name=? and class_name=? and class_left=? and class_right=?"
                ))
                .bind(item)
                .bind(&class.name)
                .bind(&class.left_op.as_ref().unwrap().name)
                .bind(&class.right_op.as_ref().unwrap().name)
                .execute(&self.pool)
                .await
                .change_context(moon_class::err::Error::RuntimeError)?;
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use moon_class::ClassExecutor;

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
                    "right<append<type<$test, pair<test, test>>, +<1, 1>>, test<test, test>>",
                ))
                .await
                .unwrap();

            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0], "2");
        })
    }
}

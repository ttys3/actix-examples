use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row, MySqlPool};

// this struct will use to receive user input
#[derive(Serialize, Deserialize)]
pub struct TodoRequest {
    pub description: String,
    pub done: bool,
}

// this struct will be used to represent database record
#[derive(Serialize, FromRow)]
pub struct Todo {
    pub id: i32,
    pub description: String,
    pub done: i8,
}

// implementation of Actix Responder for Todo struct so we can return Todo from action handler
impl Responder for Todo {

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse {
        // create response and set content type
        HttpResponse::Ok().json(&self)
    }
}

// Implementation for Todo struct, functions for read/write/update and delete todo from database
impl Todo {
    pub async fn find_all(pool: &MySqlPool) -> Result<Vec<Todo>> {
        let todos = sqlx::query!(
            r#"
            SELECT id, description, done
            FROM todos
            ORDER BY id
            "#
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|rec| Todo {
            id: rec.id,
            description: rec.description,
            done: rec.done,
        })
        .collect();

        Ok(todos)
    }

    pub async fn find_by_id(id: i32, pool: &MySqlPool) -> Result<Option<Todo>> {
        let rec = sqlx::query!(
            r#"
            SELECT id, description, done
            FROM todos
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&*pool)
        .await?;

        Ok(rec.map(|rec| Todo {
            id: rec.id,
            description: rec.description,
            done: rec.done,
        }))
    }

    pub async fn create(todo: TodoRequest, pool: &MySqlPool) -> Result<Todo> {
        let mut tx = pool.begin().await?;

        // error creating todo: Field 'id' doesn't have a default value
        // id          int                  not null
        // fixup: alter table todos modify id int auto_increment;
        sqlx::query!(
            r#"
            INSERT INTO todos (description, done)
            VALUES (?, ?)
            "#,
            todo.description,
            todo.done,
        )
        .execute(&mut tx)
        .await?;

        // mysql: error creating todo: FUNCTION db001.last_insert_rowid does not exist

        // value: Decode("mismatched types; Rust type `i32` (as SQL type INT) is not compatible with SQL type BIGINT UNSIGNED")'
        // according to https://docs.rs/sqlx/0.5.9/sqlx/mysql/types/index.html
        // BIGINT UNSIGNED is mapped to rust u64
        let row_id: u64 = sqlx::query("SELECT LAST_INSERT_ID()")
            .map(|row: MySqlRow| row.get(0))
            .fetch_one(&mut tx)
            .await?;

        let rec = sqlx::query!(
            r#"
            SELECT id, description, done
            FROM todos
            WHERE id = ?
            "#,
            row_id,
        )
        .fetch_one(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(Todo {
            id: rec.id,
            description: rec.description,
            done: rec.done,
        })
    }

    pub async fn update(
        id: i32,
        todo: TodoRequest,
        pool: &MySqlPool,
    ) -> Result<Option<Todo>> {
        let mut tx = pool.begin().await.unwrap();

        let n = sqlx::query!(
            r#"
            UPDATE todos
            SET description = ?, done = ?
            WHERE id = ?
            "#,
            todo.description,
            todo.done,
            id,
        )
        .execute(&mut tx)
        .await?;

        if n == 0 {
            return Ok(None);
        }

        // TODO: this can be replaced with RETURNING with sqlite v3.35+ and/or sqlx v0.5+
        let todo = sqlx::query!(
            r#"
            SELECT id, description, done
            FROM todos
            WHERE id = ?
            "#,
            id,
        )
        .fetch_one(&mut tx)
        .await
        .map(|rec| Todo {
            id: rec.id,
            description: rec.description,
            done: rec.done,
        })?;

        tx.commit().await.unwrap();
        Ok(Some(todo))
    }

    pub async fn delete(id: i32, pool: &MySqlPool) -> Result<u64> {
        let mut tx = pool.begin().await?;

        let n_deleted = sqlx::query!(
            r#"
            DELETE FROM todos
            WHERE id = ?
            "#,
            id,
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await?;
        Ok(n_deleted)
    }
}

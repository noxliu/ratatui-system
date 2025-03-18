use log::info;
use serde::Serialize;
use sqlx::mysql::MySqlPool;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct MmVolumeTask {
    pub id: String,
    pub launch_id: String,
    pub token_add: String,
    pub target_volume: String,
    pub do_status: String,
    pub use_wallet_type: String,
    pub remark: String,
    pub buy_rate: String,
    pub buy_per_low: String,
    pub buy_per_high: String,
    pub sell_percent: String,
    pub frequent_low: String,
    pub frequent_high: String,
    pub real_sol: String,
}

pub struct Database {
    pub pool: MySqlPool,
}

impl Database {
    // init pool
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(database_url).await?;
        Ok(Database { pool })
    }

    pub async fn get_all_mm_volume_task(&self) -> Result<Vec<MmVolumeTask>, sqlx::Error> {
        let users = sqlx::query_as::<_, MmVolumeTask>(
            "SELECT CAST(id AS CHAR) as id, 
            CAST(launch_id AS CHAR) as launch_id, 
            token_add, 
            CAST(target_volume AS CHAR) as target_volume , 
            CAST(do_status AS CHAR) as do_status , 
            CAST(use_wallet_type AS CHAR) as use_wallet_type , 
            remark, 
            CAST(buy_rate AS CHAR) as buy_rate ,
            CAST(buy_per_low AS CHAR) as buy_per_low ,
            CAST(buy_per_high AS CHAR) as buy_per_high ,
            CAST(sell_percent AS CHAR) as sell_percent ,
            CAST(frequent_low AS CHAR) as frequent_low ,
            CAST(frequent_high AS CHAR) as frequent_high ,
            CAST(real_sol AS CHAR) as real_sol
            FROM mm_volume_task",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    // pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
    //     let user = sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = ?")
    //         .bind(id)
    //         .fetch_optional(&self.pool)
    //         .await?;
    //     Ok(user)
    // }

    pub async fn insert_user(&self, name: &str, email: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("INSERT INTO users (name, email) VALUES (?, ?)")
            .bind(name)
            .bind(email)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_id())
    }

    pub async fn update_user(&self, id: i32, user_name: &str) -> Result<bool, sqlx::Error> {
        let rows_affected =
            sqlx::query("UPDATE tb_users SET userName = 'aaaaaaaaaa' WHERE accountId = 2")
                // .bind(user_name)
                // .bind(id)
                .execute(&self.pool)
                .await?
                .rows_affected();
        Ok(rows_affected > 0)
    }

    pub async fn update_record(
        &self,
        key_name: &str,
        key_value: &str,
        column_name: &str,
        column_value: &str,
    ) -> Result<bool, sqlx::Error> {
        let _sql = &format!(
            "UPDATE mm_volume_task SET {} = ? WHERE {} = ? ",
            column_name, key_name
        );
        info!("{}", _sql);
        let rows_affected = sqlx::query(_sql)
            // .bind(column_name)
            .bind(column_value)
            // .bind(key_name)
            .bind(key_value)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(rows_affected > 0)
    }

    pub async fn delete_user(&self, id: i32) -> Result<bool, sqlx::Error> {
        let rows_affected = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(rows_affected > 0)
    }
}

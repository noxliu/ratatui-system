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
    pub create_time: String,
    pub update_time: String,
    pub col1: String,
    pub col2: String,
    pub col3: String,
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct DexVolumeTask {
    pub id: String,
    pub pool_id: String,
    pub token_add: String,
    pub mm_type: String,
    pub remark: String,
    pub target_price: String,
    pub stop_price_per: String,
    pub do_status: String,
    pub buy_rate: String,
    pub buy_per_low: String,
    pub buy_per_high: String,
    pub sell_percent: String,
    pub frequent_low: String,
    pub frequent_high: String,
    pub bsdiff: String,
    pub create_time: String,
    pub update_time: String,
    pub copy: String,
    pub del: String,
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

    pub async fn get_all_mm_volume_task(
        &self,
        search_word: &str,
    ) -> Result<Vec<MmVolumeTask>, sqlx::Error> {
        let search_key_word = format!("%{}%", search_word);
        let mm_volume_tasks: Vec<MmVolumeTask> = sqlx::query_as::<_, MmVolumeTask>(
            "SELECT 
            IFNULL(CAST(id AS CHAR), '') as id, 
            IFNULL(CAST(launch_id AS CHAR), '') as launch_id, 
            IFNULL(token_add, '') as token_add , 
            IFNULL(CAST(target_volume AS CHAR), '') as target_volume , 
            IFNULL(CAST(do_status AS CHAR), '') as do_status , 
            IFNULL(CAST(use_wallet_type AS CHAR), '') as use_wallet_type , 
            IFNULL(remark, '') as remark, 
            IFNULL(CAST(buy_rate AS CHAR), '') as buy_rate ,
            IFNULL(CAST(buy_per_low AS CHAR), '') as buy_per_low ,
            IFNULL(CAST(buy_per_high AS CHAR), '') as buy_per_high ,
            IFNULL(CAST(sell_percent AS CHAR), '') as sell_percent ,
            IFNULL(CAST(frequent_low AS CHAR), '') as frequent_low ,
            IFNULL(CAST(frequent_high AS CHAR), '') as frequent_high ,
            IFNULL(CAST(real_sol AS CHAR), '') as real_sol,
            IFNULL(DATE_FORMAT(create_time, '%Y-%m-%d %H:%i:%s'), '') as create_time,
            IFNULL(DATE_FORMAT(update_time, '%Y-%m-%d %H:%i:%s'), '') as update_time,
            '' as col1,
            '' as col2,
            '' as col3
            FROM mm_volume_task WHERE token_add like ? ",
        )
        .bind(search_key_word)
        .fetch_all(&self.pool)
        .await?;
        Ok(mm_volume_tasks)
    }

    pub async fn get_all_dex_volume_task(
        &self,
        search_word: &str,
    ) -> Result<Vec<DexVolumeTask>, sqlx::Error> {
        let search_key_word = format!("%{}%", search_word);
        let dex_volume_task = sqlx::query_as::<_, DexVolumeTask>(
            "SELECT 
            IFNULL(CAST(id AS CHAR), '') as id, 
            IFNULL(pool_id, '') as pool_id , 
            IFNULL(token_add, '') as token_add , 
            IFNULL(CAST(mm_type AS CHAR), '') as mm_type , 
            IFNULL(remark, '') as remark,
            IFNULL(CAST(target_price AS CHAR), '')  as target_price ,
            IFNULL(CAST(stop_price_per AS CHAR), '')  as stop_price_per ,
            IFNULL(CAST(do_status AS CHAR), '')  as do_status , 
            IFNULL(CAST(buy_rate AS CHAR), '')  as buy_rate , 
            IFNULL(CAST(buy_per_low AS CHAR), '')  as buy_per_low , 
            IFNULL(CAST(buy_per_high AS CHAR), '')  as buy_per_high , 
            IFNULL(CAST(sell_percent AS CHAR), '')  as sell_percent , 
            IFNULL(CAST(frequent_low AS CHAR), '')  as frequent_low , 
            IFNULL(CAST(frequent_high AS CHAR), '')  as frequent_high , 
            IFNULL(CAST(bsdiff AS CHAR), '')  as bsdiff ,
            IFNULL(DATE_FORMAT(create_time, '%Y-%m-%d %H:%i:%s'), '') as create_time,
            IFNULL(DATE_FORMAT(update_time, '%Y-%m-%d %H:%i:%s'), '') as update_time,
            'copy' as copy,
            'del'  as del
            FROM dex_volume_task WHERE token_add like ? ",
        )
        .bind(search_key_word)
        .fetch_all(&self.pool)
        .await?;
        Ok(dex_volume_task)
    }

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

    pub async fn copy_dex_record_by_id(&self, key_value: &str) -> Result<bool, sqlx::Error> {
        let _sql = &format!(
            "INSERT INTO dex_volume_task (
                pool_id,
                token_add,
                mm_type,
                remark,
                target_price,
                stop_price_per,
                do_status,
                buy_rate,
                buy_per_low,
                buy_per_high,
                sell_percent,
                frequent_low,
                frequent_high,
                bsdiff
            ) 
            SELECT 
                pool_id,
                token_add,
                mm_type,
                remark,
                target_price,
                stop_price_per,
                NULL as do_status,
                buy_rate,
                buy_per_low,
                buy_per_high,
                sell_percent,
                frequent_low,
                frequent_high,
                bsdiff
            FROM dex_volume_task WHERE id = ? ",
        );
        // info!("{}", _sql);
        let rows_affected = sqlx::query(_sql)
            // .bind(column_name)
            .bind(key_value)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(rows_affected > 0)
    }

    pub async fn delete_dex_record_by_id(&self, key_value: &str) -> Result<bool, sqlx::Error> {
        let rows_affected = sqlx::query("DELETE FROM dex_volume_task WHERE id = ?")
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

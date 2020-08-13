pub mod schema;
pub mod models;

#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;
use chrono::{Utc};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

use self::models::{User, NewUser};


pub async fn create_user(conn : &PgConnection, ) -> Result<User,diesel::result::Error> {

    let new_user = NewUser{date_created: Utc::now().naive_utc(), online: false, last_login: None};
    
    diesel::insert_into(models::users::table)
    .values(&new_user)
    .get_result(conn)
}
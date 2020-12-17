pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
extern crate dotenv;

use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

use self::models::{InteractionHistory, NewInteractionHistory, NewUser, User};

pub async fn create_user(conn: &PgConnection) -> Result<User, diesel::result::Error> {
    use self::schema::users::dsl::*;

    let new_user = NewUser {
        date_created: Utc::now().naive_utc(),
        online: false,
        last_login: None,
    };

    diesel::insert_into(users)
        .values(&new_user)
        .get_result(conn)
}

/// TODO: implement a check for the game mode
pub async fn create_interaction(
    conn: &PgConnection,
    user_id: i64,
    mode: String,
) -> Result<InteractionHistory, diesel::result::Error> {
    let new_interaction = NewInteractionHistory {
        user_id,
        mode,
        start_time: Utc::now().naive_utc(),
        end_time: None,
        enjoyed_interaction: None,
    };

    //use self::schema::interaction_history::dsl::*;

    use schema::interaction_history;

    diesel::insert_into(interaction_history::table)
        .values(&new_interaction)
        .get_result(conn)
}

pub async fn list_game_modes(conn: &PgConnection) -> Result<Vec<String>, diesel::result::Error> {
    use schema::game_modes::dsl::*;

    //diesel::select().get_results(conn);
    //diesel::query_dsl::QueryDsl::distinct()

    game_modes.select(valid_mode).load::<String>(conn)

    //diesel::dsl::select(game_mode).limit(10).get_results(conn);
}

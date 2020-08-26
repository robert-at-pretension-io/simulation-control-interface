
use chrono::{ NaiveDateTime};
use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct User {
    pub id : i64,
    pub online: bool,
    pub last_login: Option<NaiveDateTime>,
    pub date_created: NaiveDateTime,
}

use super::schema::users;

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser{
    pub online: bool,
    pub last_login: Option<NaiveDateTime>,
    pub date_created: NaiveDateTime
}


#[derive(Queryable, Serialize)]
pub struct InteractionHistory {
    pub id: i64,
    pub user_id : i64,
    pub enjoyed_interaction : Option<bool>,
    pub start_time : NaiveDateTime,
    pub end_time: Option<NaiveDateTime>,
    pub mode: String //How can I place a restriction on the type of string...?
}


use super::schema::interaction_history;


#[derive(Insertable)]
#[table_name="interaction_history"]
pub struct NewInteractionHistory {
    pub user_id : i64,
    pub enjoyed_interaction : Option<bool>,
    pub start_time: NaiveDateTime,
    pub end_time: Option<NaiveDateTime>,
    pub mode: String //How can I place a restriction on the type of string...?
}


use super::schema::game_modes;
#[derive(Insertable, Queryable, Serialize)]
pub struct GameMode {
    pub valid_mode : String,
}

impl From<String> for GameMode {
    fn from(string : String) -> Self {
        GameMode {valid_mode : string}
    }
}
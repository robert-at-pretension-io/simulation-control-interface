
use chrono::{ NaiveDateTime, NaiveDate};
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


#[derive(Queryable)]
pub struct InteractionHistory {
    pub id: i64,
    pub person : i64,
    enjoyed_interaction : Option<bool>,
    start_date : NaiveDate,
    start_time: NaiveDateTime,
    end_date: Option<NaiveDate>,
    end_time: Option<NaiveDateTime>,
    mode: String //How can I place a restriction on the type of string...?
}

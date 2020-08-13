
use chrono::{ NaiveDateTime, NaiveDate};


#[derive(Queryable)]
pub struct User {
    pub id : i64,
    pub online: bool,
    pub last_login: Option<NaiveDateTime>,
    pub date_created: NaiveDateTime,
}


#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser{
    pub online: bool,
    pub last_login: Option<NaiveDateTime>,
    pub date_created: NaiveDateTime
}


#[derive(DbEnum, Debug, PartialEq)]
pub enum GameMode {
    Exploration,
    Friend,
    TwentyQuestions,
    ThisOrThat,
    JustOneMinute
}

table! {
    use super::GameModeMapping;
    use diesel::sql_types::*;
    interaction_history (id) {
        id -> Int8,
        person -> Int8,
        enjoyed_interaction -> Nullable<Bool>,
        start_date -> Timestamp,
        start_time -> Timestamp,
        end_date -> Nullable<Timestamp>,
        end_time -> Nullable<Timestamp>,
        mode -> GameModeMapping,
    }
}

table! {
    users (id) {
        id -> Int8,
        online -> Bool,
        last_login -> Nullable<Timestamp>,
        date_created -> Timestamp,
    }
}

joinable!(interaction_history -> users (person));

allow_tables_to_appear_in_same_query!(
    interaction_history,
    users,
);


#[derive(Queryable)]
pub struct InteractionHistory {
    pub id: i64,
    pub person : i64,
    enjoyed_interaction : Option<bool>,
    start_date : NaiveDate,
    start_time: NaiveDateTime,
    end_date: Option<NaiveDate>,
    end_time: Option<NaiveDateTime>,
    mode: GameMode
}

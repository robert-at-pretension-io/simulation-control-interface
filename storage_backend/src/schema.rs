table! {
    game_mode (valid_mode) {
        valid_mode -> Text,
    }
}

table! {
    interaction_history (id) {
        id -> Int8,
        person -> Int8,
        enjoyed_interaction -> Nullable<Bool>,
        start_date -> Timestamp,
        start_time -> Timestamp,
        end_date -> Nullable<Timestamp>,
        end_time -> Nullable<Timestamp>,
        mode -> Text,
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

joinable!(interaction_history -> game_mode (mode));
joinable!(interaction_history -> users (person));

allow_tables_to_appear_in_same_query!(
    game_mode,
    interaction_history,
    users,
);

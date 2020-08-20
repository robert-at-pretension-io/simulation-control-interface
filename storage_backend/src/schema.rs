table! {
    game_modes (valid_mode) {
        valid_mode -> Text,
    }
}

table! {
    interaction_history (id, user_id) {
        id -> Int8,
        user_id -> Int8,
        enjoyed_interaction -> Nullable<Bool>,
        start_time -> Timestamp,
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

joinable!(interaction_history -> game_modes (mode));
joinable!(interaction_history -> users (user_id));

allow_tables_to_appear_in_same_query!(
    game_modes,
    interaction_history,
    users,
);

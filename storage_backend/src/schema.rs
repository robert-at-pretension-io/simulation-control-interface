table! {
    categorical_types (id) {
        approved -> Bool,
        question_description -> Nullable<Varchar>,
        comma_separated_options -> Nullable<Varchar>,
        id -> Int8,
    }
}

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
    numeric_types (id) {
        id -> Int8,
        lower_bound -> Float4,
        upper_bound -> Float4,
        stepsize -> Float4,
        question_description -> Varchar,
        approved -> Bool,
    }
}

table! {
    user_question_responses (id, user_id) {
        id -> Int8,
        user_id -> Int8,
        response_time -> Timestamp,
        numeric_type_id -> Int8,
        categorical_type_id -> Int8,
        is_hidden -> Nullable<Bool>,
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
joinable!(user_question_responses -> categorical_types (categorical_type_id));
joinable!(user_question_responses -> numeric_types (numeric_type_id));
joinable!(user_question_responses -> users (user_id));

allow_tables_to_appear_in_same_query!(
    categorical_types,
    game_modes,
    interaction_history,
    numeric_types,
    user_question_responses,
    users,
);

table! {
    users (id) {
        id -> Int8,
        online -> Bool,
        last_login -> Nullable<Timestamp>,
        date_created -> Timestamp,
    }
}

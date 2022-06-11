table! {
    acls (id) {
        id -> Integer,
        subject -> Text,
        action -> Text,
        path -> Text,
        user -> Text,
        create_by -> Text,
        create_time -> Text,
        decision -> Text,
    }
}

table! {
    job_info (id) {
        id -> Integer,
        uuid -> Text,
        status -> Text,
        output -> Text,
        create_time -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    acls,
    job_info,
);

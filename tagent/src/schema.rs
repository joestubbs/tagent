table! {
    acls (id) {
        id -> Nullable<Integer>,
        subject -> Text,
        action -> Text,
        path -> Text,
        user -> Text,
        create_by -> Text,
        create_time -> Text,
    }
}

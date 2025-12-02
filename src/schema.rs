// @generated automatically by Diesel CLI.

diesel::table! {
    game_versions (build_number) {
        build_number -> Integer,
        version -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    java_versions (version) {
        version -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    launcher_versions (version) {
        version -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    update_channels (channel) {
        channel -> Text,
        launcher_version_id -> Text,
        java_version_id -> Text,
        game_version_id -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::joinable!(update_channels -> game_versions (game_version_id));
diesel::joinable!(update_channels -> java_versions (java_version_id));
diesel::joinable!(update_channels -> launcher_versions (launcher_version_id));

diesel::allow_tables_to_appear_in_same_query!(
    game_versions,
    java_versions,
    launcher_versions,
    update_channels,
);

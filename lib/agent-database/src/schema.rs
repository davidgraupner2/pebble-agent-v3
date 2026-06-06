use crate::schema::sql_types::ConnectionStatsStatus;

// Custom SQL types for enums
pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(sqlite_type(name = "Text"))]
    pub struct EventStatus;
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(sqlite_type(name = "Text"))]
    pub struct ConnectionStringStatus;
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(sqlite_type(name = "Text"))]
    pub struct ConnectionStatsStatus;
}

diesel::table! {
    use diesel::sql_types::*;

    registration (id) {
        id -> Integer,
        agent_id -> Text,
        jti -> Text,
        source -> Text,
        expires_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    agent_identities (id) {
        id -> Integer,
        agent_uuid -> Text,
        pubkey_fingerprint -> Text,
        pubkey_b64u -> Text,
        agent_id -> Text,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    registration_challenges (id) {
        id -> Integer,
        challenge_id -> Text,
        nonce_b64u -> Text,
        pubkey_fingerprint_b64u -> Text,
        registration_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    agent_jwt (id) {
        id -> Integer,
        registration_id -> Text,
        jti -> Text,
        status -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ConnectionStringStatus;

    connection_strings (id) {
        id -> Integer,
        value -> Text,
        description -> Nullable<Text>,
        source -> Text,
        status -> ConnectionStringStatus,
        environment -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::EventStatus;

    events (id) {
        id -> Integer,
        event_type -> Text,
        aggregate_type -> Text,
        aggregate_id -> Text,
        payload -> Text,
        metadata -> Nullable<Text>,
        status -> EventStatus,
        retry_count -> Integer,
        processed_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ConnectionStatsStatus;

    connection_stats (id) {
        id -> Integer,
        endpoint -> Text,
        status -> ConnectionStatsStatus,
        connected_at -> Timestamp,
        disconnected_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    function_hashes (id) {
        id -> Integer,
        function_hash -> Text,
        description -> Nullable<Text>,
        source -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    secrets (id) {
        id -> Integer,
        name -> Text,
        secret_type -> Text,
        description -> Nullable<Text>,
        value -> Text,
        source -> Text,
        ephemeral_key -> Nullable<Text>,
        nonce -> Nullable<Text>,
        encryption_key_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    cache_tags (cache_id, tag_id) {
        cache_id -> Integer,
        tag_id -> Integer
    }
}

diesel::table! {
    secret_tags (secret_id, tag_id) {
        secret_id -> Integer,
        tag_id -> Integer
    }
}

diesel::joinable!(secrets -> encryption_keys (encryption_key_id));
diesel::joinable!(cache_tags -> tags (tag_id));
diesel::joinable!(cache_tags -> cache (cache_id));
diesel::joinable!(secret_tags -> tags (tag_id));
diesel::joinable!(secret_tags -> secrets (secret_id));

diesel::allow_tables_to_appear_in_same_query!(encryption_keys, secrets, tags);

diesel::table! {
    cache (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        #[sql_name = "type"]
        type_ -> Text,
        value -> Text,
        source -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        expires_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    encryption_keys (id) {
        id -> Integer,
        name -> Text,
        enabled -> Integer,
        public_key -> Text,
        source -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    properties (id) {
        id -> Integer,
        agent_uuid -> Nullable<Text>,
        name -> Text,
        #[sql_name = "type"]
        type_ -> Text,
        source -> Text,
        description -> Nullable<Text>,
        value_int -> Nullable<Integer>,
        value_string -> Nullable<Text>,
        value_bool -> Nullable<Integer>,
        value_json -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tags (id) {
        id -> Integer,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    job_logs (id) {
        id -> Integer,
        job_id -> Text,
        name -> Text,
        hash -> Text,
        source -> Text,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(cache_tags, tags);
diesel::allow_tables_to_appear_in_same_query!(secret_tags, tags);

diesel::allow_tables_to_appear_in_same_query!(
    connection_stats,
    connection_strings,
    events,
    function_hashes,
    properties,
    tags,
);

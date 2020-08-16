table! {
    blocked_guilds (guild) {
        guild -> BigInt,
    }
}

table! {
    blocked_users (user) {
        user -> BigInt,
    }
}

table! {
    messages (id) {
        id -> BigInt,
        game_id -> Integer,
        mod_id -> Integer,
        created_at -> Timestamp,
    }
}

table! {
    settings (guild) {
        guild -> BigInt,
        game -> Nullable<Integer>,
        prefix -> Nullable<Text>,
    }
}

table! {
    subscriptions (game, channel, tags) {
        game -> Integer,
        channel -> BigInt,
        tags -> Text,
        guild -> Nullable<BigInt>,
        events -> Integer,
    }
}

table! {
    subscriptions_exclude_mods (game, channel, mod_id) {
        game -> Integer,
        channel -> BigInt,
        guild -> Nullable<BigInt>,
        mod_id -> Integer,
    }
}

table! {
    subscriptions_exclude_users (game, channel, user) {
        game -> Integer,
        channel -> BigInt,
        guild -> Nullable<BigInt>,
        user -> Text,
    }
}

table! {
    users (id) {
        id -> BigInt,
        access_token -> Text,
        refresh_token -> Text,
        expired_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(
    blocked_guilds,
    blocked_users,
    messages,
    settings,
    subscriptions,
    subscriptions_exclude_mods,
    subscriptions_exclude_users,
    users,
);

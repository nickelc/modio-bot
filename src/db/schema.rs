// @generated automatically by Diesel CLI.

diesel::table! {
    blocked_guilds (guild) {
        guild -> BigInt,
    }
}

diesel::table! {
    blocked_users (user) {
        user -> BigInt,
    }
}

diesel::table! {
    settings (guild) {
        guild -> BigInt,
        game -> Nullable<Integer>,
    }
}

diesel::table! {
    subscriptions (game, channel, tags) {
        game -> Integer,
        channel -> BigInt,
        tags -> Text,
        guild -> BigInt,
        events -> Integer,
    }
}

diesel::table! {
    subscriptions_exclude_mods (game, channel, mod_id) {
        game -> Integer,
        channel -> BigInt,
        guild -> BigInt,
        mod_id -> Integer,
    }
}

diesel::table! {
    subscriptions_exclude_users (game, channel, user) {
        game -> Integer,
        channel -> BigInt,
        guild -> BigInt,
        user -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    blocked_guilds,
    blocked_users,
    settings,
    subscriptions,
    subscriptions_exclude_mods,
    subscriptions_exclude_users,
);

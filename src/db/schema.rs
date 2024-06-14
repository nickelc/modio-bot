// @generated automatically by Diesel CLI.

diesel::table! {
    games (id) {
        id -> BigInt,
        name -> Text,
        name_id -> Text,
        api_access_options -> Integer,
        autocomplete -> Bool,
    }
}

diesel::table! {
    settings (guild) {
        guild -> BigInt,
        game -> Nullable<BigInt>,
    }
}

diesel::table! {
    subscriptions (game, channel, tags) {
        game -> BigInt,
        channel -> BigInt,
        tags -> Text,
        guild -> BigInt,
        events -> Integer,
    }
}

diesel::table! {
    subscriptions_exclude_mods (game, channel, mod_id) {
        game -> BigInt,
        channel -> BigInt,
        guild -> BigInt,
        mod_id -> BigInt,
    }
}

diesel::table! {
    subscriptions_exclude_users (game, channel, user) {
        game -> BigInt,
        channel -> BigInt,
        guild -> BigInt,
        user -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    games,
    settings,
    subscriptions,
    subscriptions_exclude_mods,
    subscriptions_exclude_users,
);

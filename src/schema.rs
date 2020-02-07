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
    settings (guild) {
        guild -> BigInt,
        game -> Nullable<Integer>,
        prefix -> Nullable<Text>,
    }
}

table! {
    subscriptions (game, channel) {
        game -> Integer,
        channel -> BigInt,
        guild -> Nullable<BigInt>,
        events -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    blocked_guilds,
    blocked_users,
    settings,
    subscriptions,
);

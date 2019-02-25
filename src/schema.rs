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
    }
}

allow_tables_to_appear_in_same_query!(
    settings,
    subscriptions,
);

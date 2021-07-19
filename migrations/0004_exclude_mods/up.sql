CREATE TABLE subscriptions_exclude_mods (
    game INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild INTEGER,
    mod_id INTEGER NOT NULL,
    PRIMARY KEY (game, channel)
);

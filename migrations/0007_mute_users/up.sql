CREATE TABLE subscriptions_exclude_users (
    game INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild INTEGER,
    user TEXT NOT NULL,
    PRIMARY KEY (game, channel, user)
);

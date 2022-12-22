CREATE TABLE subscriptions (
    game    INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild   INTEGER,
    PRIMARY KEY (game, channel)
);

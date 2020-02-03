CREATE TABLE subscriptions_tmp (
    game INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild INTEGER,
    PRIMARY KEY (game, channel)
);

INSERT INTO subscriptions_tmp SELECT game, channel, guild FROM subscriptions;
DROP TABLE subscriptions;
ALTER TABLE subscriptions_tmp RENAME TO subscriptions;

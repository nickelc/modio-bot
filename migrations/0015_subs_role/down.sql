CREATE TABLE subscriptions_tmp (
    game    BIGINT NOT NULL,
    channel BIGINT NOT NULL,
    tags    TEXT NOT NULL DEFAULT "",
    guild   BIGINT NOT NULL,
    events  INTEGER NOT NULL DEFAULT 3,
    explicit BOOLEAN NOT NULL DEFAULT 1,
    PRIMARY KEY(game, channel, tags)
);

INSERT INTO subscriptions_tmp (game, channel, tags, guild, events, explicit)
    SELECT game, channel, tags, guild, events, explicit FROM subscriptions;
DROP TABLE subscriptions;
ALTER TABLE subscriptions_tmp RENAME TO subscriptions;

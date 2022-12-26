CREATE TABLE blocked_guilds_tmp (
    guild BIGINT PRIMARY KEY NOT NULL
);

INSERT INTO blocked_guilds_tmp SELECT * FROM blocked_guilds;
DROP TABLE blocked_guilds;
ALTER TABLE blocked_guilds_tmp RENAME TO blocked_guilds;

CREATE TABLE blocked_users_tmp (
    user BIGINT PRIMARY KEY NOT NULL
);

INSERT INTO blocked_users_tmp SELECT * FROM blocked_users;
DROP TABLE blocked_users;
ALTER TABLE blocked_users_tmp RENAME TO blocked_users;

CREATE TABLE settings_tmp (
    guild BIGINT PRIMARY KEY NOT NULL,
    game INTEGER NULL
);

INSERT INTO settings_tmp (guild, game) SELECT guild, game FROM settings;
DROP TABLE settings;
ALTER TABLE settings_tmp RENAME TO settings;

CREATE TABLE subscriptions_tmp (
    game    INTEGER NOT NULL,
    channel BIGINT NOT NULL,
    tags    TEXT NOT NULL DEFAULT "",
    guild   BIGINT NOT NULL,
    events  INTEGER NOT NULL DEFAULT 3,
    PRIMARY KEY(game, channel, tags)
);

INSERT INTO subscriptions_tmp (game, channel, tags, guild, events)
    SELECT game, channel, tags, guild, events FROM subscriptions;
DROP TABLE subscriptions;
ALTER TABLE subscriptions_tmp RENAME TO subscriptions;

CREATE TABLE subscriptions_exclude_mods_tmp (
    game    INTEGER NOT NULL,
    channel BIGINT NOT NULL,
    guild   BIGINT NOT NULL,
    mod_id  INTEGER NOT NULL,
    PRIMARY KEY (game, channel, mod_id)
);

INSERT INTO subscriptions_exclude_mods_tmp (game, channel, guild, mod_id)
    SELECT game, channel, guild, mod_id FROM subscriptions_exclude_mods;
DROP TABLE subscriptions_exclude_mods;
ALTER TABLE subscriptions_exclude_mods_tmp RENAME TO subscriptions_exclude_mods;

CREATE TABLE subscriptions_exclude_users_tmp (
    game    INTEGER NOT NULL,
    channel BIGINT NOT NULL,
    guild   BIGINT NOT NULL,
    user    TEXT NOT NULL,
    PRIMARY KEY (game, channel, user)
);

INSERT INTO subscriptions_exclude_users_tmp (game, channel, guild, user)
    SELECT game, channel, guild, user FROM subscriptions_exclude_users;
DROP TABLE subscriptions_exclude_users;
ALTER TABLE subscriptions_exclude_users_tmp RENAME TO subscriptions_exclude_users;

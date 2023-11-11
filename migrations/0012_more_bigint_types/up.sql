CREATE TABLE settings_tmp (
    guild BIGINT PRIMARY KEY NOT NULL,
    game BIGINT NULL
);

INSERT INTO settings_tmp (guild, game) SELECT guild, game FROM settings;
DROP TABLE settings;
ALTER TABLE settings_tmp RENAME TO settings;

CREATE TABLE subscriptions_tmp (
    game    BIGINT NOT NULL,
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
    game    BIGINT NOT NULL,
    channel BIGINT NOT NULL,
    guild   BIGINT NOT NULL,
    mod_id  BIGINT NOT NULL,
    PRIMARY KEY (game, channel, mod_id)
);

INSERT INTO subscriptions_exclude_mods_tmp (game, channel, guild, mod_id)
    SELECT game, channel, guild, mod_id FROM subscriptions_exclude_mods;
DROP TABLE subscriptions_exclude_mods;
ALTER TABLE subscriptions_exclude_mods_tmp RENAME TO subscriptions_exclude_mods;

CREATE TABLE subscriptions_exclude_users_tmp (
    game    BIGINT NOT NULL,
    channel BIGINT NOT NULL,
    guild   BIGINT NOT NULL,
    user    TEXT NOT NULL,
    PRIMARY KEY (game, channel, user)
);

INSERT INTO subscriptions_exclude_users_tmp (game, channel, guild, user)
    SELECT game, channel, guild, user FROM subscriptions_exclude_users;
DROP TABLE subscriptions_exclude_users;
ALTER TABLE subscriptions_exclude_users_tmp RENAME TO subscriptions_exclude_users;

CREATE TABLE games_tmp (
    id      BIGINT PRIMARY KEY NOT NULL,
    name    TEXT NOT NULL,
    name_id TEXT NOT NULL,
    api_access_options  INTEGER NOT NULL,
    autocomplete        BOOLEAN NOT NULL AS (api_access_options & 1)
);

INSERT INTO games_tmp (id,name,name_id,api_access_options)
    SELECT id, name, name_id, api_access_options FROM games;
DROP TABLE games;
ALTER TABLE games_tmp RENAME TO games;

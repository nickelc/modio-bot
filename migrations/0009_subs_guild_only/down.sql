CREATE TABLE subscriptions_old (
    game    INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    tags    TEXT NOT NULL DEFAULT "",
    guild   INTEGER,
    events  INTEGER NOT NULL DEFAULT 3,
    PRIMARY KEY(game, channel, tags)
);

INSERT INTO subscriptions_old (game, channel, tags, guild, events)
    SELECT game, channel, tags, guild, events FROM subscriptions;

DROP TABLE subscriptions;
ALTER TABLE subscriptions_old RENAME TO subscriptions;


CREATE TABLE subscriptions_exclude_mods_old (
    game    INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild   INTEGER,
    mod_id  INTEGER NOT NULL,
    PRIMARY KEY (game, channel, mod_id)
);

INSERT INTO subscriptions_exclude_mods_old (game, channel, guild, mod_id)
    SELECT game, channel, guild, mod_id FROM subscriptions_exclude_mods;

DROP TABLE subscriptions_exclude_mods;
ALTER TABLE subscriptions_exclude_mods_old RENAME TO subscriptions_exclude_mods;


CREATE TABLE subscriptions_exclude_users_old (
    game    INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild   INTEGER,
    user    TEXT NOT NULL,
    PRIMARY KEY (game, channel, user)
);

INSERT INTO subscriptions_exclude_users_old (game, channel, guild, user)
    SELECT game, channel, guild, user FROM subscriptions_exclude_users WHERE guild IS NOT NULL;

DROP TABLE subscriptions_exclude_users;
ALTER TABLE subscriptions_exclude_users_old RENAME TO subscriptions_exclude_users;

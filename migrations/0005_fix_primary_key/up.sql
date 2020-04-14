CREATE TABLE subscriptions_exclude_mods_new (
    game INTEGER NOT NULL,
    channel INTEGER NOT NULL,
    guild INTEGER,
    mod_id INTEGER NOT NULL,
    PRIMARY KEY (game, channel, mod_id)
);

INSERT INTO subscriptions_exclude_mods_new (game, channel, guild, mod_id)
   SELECT game, channel, guild, mod_id FROM subscriptions_exclude_mods;

DROP TABLE subscriptions_exclude_mods;

ALTER TABLE subscriptions_exclude_mods_new RENAME TO subscriptions_exclude_mods;

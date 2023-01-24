CREATE TABLE settings_new (
    guild BIGINT PRIMARY KEY NOT NULL,
    game INTEGER NULL
);

INSERT INTO settings_new (guild, game) SELECT guild, game FROM settings;
DROP TABLE settings;
ALTER TABLE settings_new RENAME TO settings;

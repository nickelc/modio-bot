CREATE TABLE "subscriptions_tags" (
	"game"	INTEGER NOT NULL,
	"channel"	INTEGER NOT NULL,
	"tags"	TEXT NOT NULL DEFAULT "",
	"guild"	INTEGER,
	"events"	INTEGER NOT NULL DEFAULT 3,
	PRIMARY KEY("game","channel","tags")
);

INSERT INTO subscriptions_tags (game, channel, guild, events)
    SELECT game, channel, guild, events FROM subscriptions;

DROP TABLE subscriptions;

ALTER TABLE subscriptions_tags RENAME TO subscriptions;

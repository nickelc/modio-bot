CREATE TABLE "subscriptions_tmp" (
	"game"	INTEGER NOT NULL,
	"channel"	INTEGER NOT NULL,
	"guild"	INTEGER,
	"events"	INTEGER DEFAULT 3,
	PRIMARY KEY("game","channel")
);

INSERT INTO subscriptions_tmp (game, channel, guild, events)
    SELECT game, channel, guild, events FROM subscriptions;

DROP TABLE subscriptions;

ALTER TABLE subscriptions_tmp RENAME TO subscriptions;

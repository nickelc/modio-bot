CREATE TABLE "users" (
	"id"	INTEGER NOT NULL,
	"access_token"	TEXT NOT NULL,
	"refresh_token"	TEXT NOT NULL,
	"expired_at"	TIMESTAMP NOT NULL,
	PRIMARY KEY("id")
)

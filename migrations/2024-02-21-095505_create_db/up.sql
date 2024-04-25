-- Your SQL goes here
CREATE TABLE "videos" (
    "id" INTEGER PRIMARY KEY ASC AUTOINCREMENT NOT NULL,
    "uid" VARCHAR(11),
    "link" VARCHAR(127),
    "title" VARCHAR(255),
    "author" VARCHAR(255),
    "duration" INTEGER,
    "description" VARCHAR(255),
    "thumbnail_path" VARCHAR(255),
    "date" INTEGER,
    "other" BLOB
)

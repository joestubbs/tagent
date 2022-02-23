-- Your SQL goes here
CREATE TABLE "acls" (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    subject TEXT NOT NULL,
    action TEXT NOT NULL,
    path TEXT NOT NULL,
    user TEXT NOT NULL,
    create_by TEXT NOT NULL,
    create_time TEXT NOT NULL
);
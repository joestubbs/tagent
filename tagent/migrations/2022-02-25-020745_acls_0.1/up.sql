-- Your SQL goes here
ALTER TABLE "acls"
ADD COLUMN decision TEXT NOT NULL DEFAULT "Allow";
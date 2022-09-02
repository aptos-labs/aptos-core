-- Your SQL goes here
ALTER TABLE move_resources
ADD COLUMN type_str text NOT NULL;
ALTER TABLE events
ADD COLUMN type_str text NOT NULL;
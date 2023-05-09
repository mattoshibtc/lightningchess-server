-- Add migration script here
ALTER TABLE lightningchess_transaction ADD COLUMN created_on TIMESTAMP without time zone default (now() at time zone 'utc');
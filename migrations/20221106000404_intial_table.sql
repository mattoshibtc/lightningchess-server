-- Add migration script here
CREATE TABLE IF NOT EXISTS challenge (
  id serial PRIMARY KEY,
	username VARCHAR (255) NOT NULL,
	time_limit INT,
	opponent_time_limit INT,
	increment INT,
	color VARCHAR (10),
	sats BIGINT,
	opp_username VARCHAR (255) NOT NULL,
	status VARCHAR (255),
	lichess_challenge_id VARCHAR (255),
	created_on TIMESTAMP without time zone default (now() at time zone 'utc'),
	expire_after INT
);

CREATE INDEX IF NOT EXISTS challenge_username_idx ON challenge(username);
CREATE INDEX IF NOT EXISTS challenge_opp_username_idx ON challenge(opp_username);

CREATE TABLE IF NOT EXISTS lightningchess_balance (
  balance_id serial PRIMARY KEY,
	username VARCHAR (255) NOT NULL UNIQUE,
	balance BIGINT
);

CREATE INDEX IF NOT EXISTS lightningchess_balance_username_idx ON lightningchess_balance(username);

CREATE TABLE IF NOT EXISTS lightningchess_transaction (
  transaction_id serial PRIMARY KEY,
	username VARCHAR (255) NOT NULL,
	ttype VARCHAR (255) NOT NULL,
	detail VARCHAR (255) NOT NULL,
  amount BIGINT,
  state VARCHAR (255),
  preimage VARCHAR (255),
  payment_addr VARCHAR (255),
  payment_request VARCHAR (510),
  payment_hash VARCHAR (255) UNIQUE,
  lichess_challenge_id VARCHAR (255) UNIQUE
);

CREATE INDEX IF NOT EXISTS lightningchess_transaction_username_idx ON lightningchess_transaction(username);




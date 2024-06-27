-- Add migration script here
ALTER TABLE game_log ADD COLUMN id INTEGER;
CREATE SEQUENCE game_log_id_seq OWNED BY game_log.id;
ALTER TABLE game_log ALTER COLUMN id SET DEFAULT nextval('game_log_id_seq');
UPDATE game_log SET id = nextval('game_log_id_seq');

ALTER TABLE game_log DROP CONSTRAINT game_log_pkey;
ALTER TABLE game_log ADD PRIMARY KEY (id);
-- CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS game_config (
    game_id TEXT NOT NULL PRIMARY KEY,
    num_players SMALLINT NOT NULL,
    hand_size SMALLINT NOT NULL,
    num_fuses SMALLINT NOT NULL,
    num_hints SMALLINT NOT NULL,
    starting_player SMALLINT NOT NULL,
    seed BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS player (
    game_id TEXT NOT NULL REFERENCES game_config(game_id),
    player_index SMALLINT NOT NULL,
    display_name TEXT NOT NULL,
    PRIMARY KEY(game_id, player_index)
);

CREATE TABLE IF NOT EXISTS game_log (
    game_id TEXT NOT NULL REFERENCES game_config(game_id),
    turn_id SMALLINT NOT NULL, 
    player_index SMALLINT NOT NULL,
    player_action JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY(game_id, turn_id)
);
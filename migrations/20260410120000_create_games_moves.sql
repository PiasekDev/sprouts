CREATE TYPE game_status AS ENUM ('waiting', 'active', 'finished');

CREATE TABLE games (
	id UUID PRIMARY KEY DEFAULT uuidv7(),
	status game_status NOT NULL,
	player1_user_id UUID NOT NULL REFERENCES users(id),
	player2_user_id UUID REFERENCES users(id),
	current_turn_user_id UUID REFERENCES users(id),
	winner_user_id UUID REFERENCES users(id),
	join_code TEXT NOT NULL UNIQUE,
	board_state_jsonb JSONB NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	CONSTRAINT games_join_code_not_empty CHECK (join_code <> ''),
	CONSTRAINT games_player2_not_player1 CHECK (
		player2_user_id IS NULL OR player2_user_id <> player1_user_id
	),
	CONSTRAINT games_current_turn_is_participant CHECK (
		current_turn_user_id IS NULL
		OR current_turn_user_id = player1_user_id
		OR current_turn_user_id = player2_user_id
	),
	CONSTRAINT games_winner_is_participant CHECK (
		winner_user_id IS NULL
		OR winner_user_id = player1_user_id
		OR winner_user_id = player2_user_id
	)
);

CREATE INDEX games_player1_user_id_idx ON games (player1_user_id);
CREATE INDEX games_player2_user_id_idx ON games (player2_user_id);
CREATE INDEX games_status_idx ON games (status);

CREATE TABLE moves (
	id UUID PRIMARY KEY DEFAULT uuidv7(),
	game_id UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
	player_user_id UUID NOT NULL REFERENCES users(id),
	move_number INTEGER NOT NULL,
	start_spot_id INTEGER NOT NULL,
	end_spot_id INTEGER NOT NULL,
	path_jsonb JSONB NOT NULL,
	new_spot_jsonb JSONB NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	CONSTRAINT moves_move_number_positive CHECK (move_number > 0),
	CONSTRAINT moves_start_spot_id_positive CHECK (start_spot_id > 0),
	CONSTRAINT moves_end_spot_id_positive CHECK (end_spot_id > 0),
	CONSTRAINT moves_game_move_number_unique UNIQUE (game_id, move_number)
);

CREATE INDEX moves_game_id_move_number_idx ON moves (game_id, move_number);

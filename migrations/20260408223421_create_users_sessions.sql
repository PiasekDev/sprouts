CREATE TABLE users (
	id UUID PRIMARY KEY DEFAULT uuidv7(),
	username TEXT NOT NULL UNIQUE,
	password_hash TEXT NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	CONSTRAINT users_username_not_empty CHECK (username <> ''),
	CONSTRAINT users_username_min_length CHECK (char_length(username) >= 3),
	CONSTRAINT users_password_hash_not_empty CHECK (password_hash <> '')
);

CREATE TABLE sessions (
	id UUID PRIMARY KEY DEFAULT uuidv7(),
	user_id UUID NOT NULL REFERENCES users(id),
	token_hash TEXT NOT NULL UNIQUE,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	last_used_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
	expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
	CONSTRAINT sessions_token_hash_not_empty CHECK (token_hash <> ''),
	CONSTRAINT sessions_expires_after_creation CHECK (expires_at > created_at)
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

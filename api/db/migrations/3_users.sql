create table "users" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email text NOT NULL UNIQUE,
    first_name text NOT NULL,
    last_name text NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
create table "user_sessions" (
    id integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id integer NOT NULL,
    session_token_p1 text NOT NULL,
    session_token_p2 text NOT NULL,
    created_at integer NOT NULL,
    expires_at integer NOT NULL
);
create table "oauth2_state_storage" (
    id integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    csrf_state text NOT NULL,
    nonce text NOT NULL,
    return_url text NOT NULL
);
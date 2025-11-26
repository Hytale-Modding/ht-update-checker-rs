PRAGMA temp_store = MEMORY;
PRAGMA foreign_keys = ON;
PRAGMA mmap_size = 2000000;

CREATE TABLE launcher_versions (
    version TEXT NOT NULL PRIMARY KEY,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE java_versions (
    version TEXT NOT NULL PRIMARY KEY,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE game_versions (
    build_number INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    version TEXT NOT NULL,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE update_channels (
    channel TEXT NOT NULL PRIMARY KEY,

    launcher_version_id TEXT NOT NULL,
    java_version_id TEXT NOT NULL,
    game_version_id INTEGER NOT NULL,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (launcher_version_id)
        REFERENCES launcher_versions (version) ON DELETE RESTRICT ON UPDATE CASCADE,

    FOREIGN KEY (java_version_id)
        REFERENCES java_versions (version) ON DELETE RESTRICT ON UPDATE CASCADE,

    FOREIGN KEY (game_version_id)
        REFERENCES game_versions (version) ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE TABLE credentials (
    id TEXT NOT NULL PRIMARY KEY,
    token TEXT NOT NULL,
    token_type TEXT NOT NULL,
    scopes TEXT NOT NULL DEFAULT 'openid',

    expires_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    refresh_token TEXT,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

pragma optimize;

CREATE TABLE online_operators (
  id uuid PRIMARY KEY,
  telegram_id bigint NOT NULL UNIQUE CHECK (telegram_id > 0 AND telegram_id <= 9007199254740991),
  first_name text NOT NULL,
  last_name text,
  username text,
  dog_tag text NOT NULL UNIQUE CHECK (dog_tag ~ '^K-[0-9]{3}-[0-9]{3}$'),
  profile jsonb NOT NULL CHECK (jsonb_typeof(profile) = 'object'),
  appearance jsonb NOT NULL CHECK (jsonb_typeof(appearance) = 'object'),
  loadout jsonb NOT NULL CHECK (jsonb_typeof(loadout) = 'object'),
  version integer NOT NULL DEFAULT 1 CHECK (version > 0),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL
);
-- A shared allocator lock serializes registration, including repeated same-user logins.
CREATE TABLE online_tag_counters (
  id integer PRIMARY KEY CHECK (id = 1),
  last_tag text
);
INSERT INTO online_tag_counters (id) VALUES (1);
CREATE TABLE online_sessions (
  token_hash text PRIMARY KEY CHECK (token_hash ~ '^[0-9a-f]{64}$'),
  operator_id uuid NOT NULL REFERENCES online_operators(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz,
  CHECK (expires_at > created_at)
);
CREATE INDEX online_sessions_operator ON online_sessions(operator_id);
CREATE INDEX online_sessions_expiry ON online_sessions(expires_at);

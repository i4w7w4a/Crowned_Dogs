CREATE TABLE online_browser_challenges (
  id text PRIMARY KEY CHECK(id ~ '^[A-Za-z0-9_-]{32}$'),
  nonce_hash text NOT NULL CHECK(nonce_hash ~ '^[0-9a-f]{64}$'),
  invite_token text,
  identity jsonb,
  created_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL,
  consumed_at timestamptz,
  CHECK(expires_at>created_at)
);
CREATE INDEX online_browser_challenges_expiry ON online_browser_challenges(expires_at);

-- Pilot: short serialized transactions; never hold this lock during network I/O.
CREATE TABLE online_game_gate (id integer PRIMARY KEY CHECK(id=1));
INSERT INTO online_game_gate VALUES(1);
CREATE TABLE online_matches (
  id uuid PRIMARY KEY, snapshot jsonb NOT NULL,
  phase text NOT NULL CHECK(phase IN ('loading','active','finished','cancelled')),
  deadline timestamptz, version integer NOT NULL CHECK(version>0),
  created_at timestamptz NOT NULL, updated_at timestamptz NOT NULL
);
CREATE INDEX online_matches_due ON online_matches(deadline) WHERE phase IN ('loading','active');
CREATE TABLE online_match_players (
  match_id uuid NOT NULL REFERENCES online_matches(id), operator_id uuid NOT NULL REFERENCES online_operators(id),
  side integer NOT NULL CHECK(side IN (0,1)), PRIMARY KEY(match_id,operator_id), UNIQUE(match_id,side)
);
CREATE INDEX online_match_players_operator ON online_match_players(operator_id);
CREATE TABLE online_invites (
  id uuid PRIMARY KEY, token text NOT NULL UNIQUE, host_id uuid NOT NULL REFERENCES online_operators(id), host_snapshot jsonb NOT NULL,
  status text NOT NULL CHECK(status IN ('open','accepted','cancelled','expired')), version integer NOT NULL DEFAULT 1,
  match_id uuid REFERENCES online_matches(id), expires_at timestamptz NOT NULL,
  chat_id text NOT NULL, thread_id integer NOT NULL CHECK(thread_id>0), message_id bigint,
  delivery_status text NOT NULL DEFAULT 'pending' CHECK(delivery_status IN ('pending','sending','published','failed')),
  delivery_error text, created_at timestamptz NOT NULL, updated_at timestamptz NOT NULL
);
CREATE INDEX online_invites_due ON online_invites(expires_at) WHERE status='open';
CREATE TABLE online_player_slots (
  operator_id uuid PRIMARY KEY REFERENCES online_operators(id), invite_id uuid REFERENCES online_invites(id), match_id uuid REFERENCES online_matches(id),
  CHECK ((invite_id IS NOT NULL)::int+(match_id IS NOT NULL)::int=1)
);
CREATE TABLE online_command_receipts (
  operator_id uuid NOT NULL REFERENCES online_operators(id), command_id uuid NOT NULL, request_hash text NOT NULL,
  status integer NOT NULL, response jsonb NOT NULL, created_at timestamptz NOT NULL, PRIMARY KEY(operator_id,command_id)
);
CREATE TABLE online_reward_ledger (
  match_id uuid NOT NULL REFERENCES online_matches(id), operator_id uuid NOT NULL REFERENCES online_operators(id),
  reward jsonb NOT NULL, created_at timestamptz NOT NULL, PRIMARY KEY(match_id,operator_id)
);
CREATE TABLE online_outbox (
  id uuid PRIMARY KEY, sequence bigserial UNIQUE, invite_id uuid NOT NULL REFERENCES online_invites(id),
  kind text NOT NULL CHECK(kind IN ('publish','update')),
  status text NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','sending','published','failed')),
  attempts integer NOT NULL DEFAULT 0, available_at timestamptz NOT NULL, lease_id uuid UNIQUE, lease_until timestamptz,
  last_error text, created_at timestamptz NOT NULL, updated_at timestamptz NOT NULL
);
CREATE INDEX online_outbox_due ON online_outbox(status,available_at);
CREATE INDEX online_outbox_invite ON online_outbox(invite_id,sequence);

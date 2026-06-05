Create table registration_challenges (
  id integer not null primary key autoincrement,
  challenge_id text not null unique,
  nonce_b64u text not null,
  pubkey_fingerprint_b64u text not null,
  registration_id text not null,
  created_at timestamp not null default current_timestamp
);

create index idx_registration_challenges_challenge_id on registration_challenges(challenge_id);


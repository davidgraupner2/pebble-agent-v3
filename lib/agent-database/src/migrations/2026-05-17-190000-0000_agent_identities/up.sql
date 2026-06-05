Create table agent_identities (
  id integer not null primary key autoincrement,
  agent_uuid text not null unique,
  pubkey_fingerprint text not null unique,
  pubkey_b64u text not null,
  agent_id text not null,
  status text not null default 'active',
  created_at timestamp not null default current_timestamp,
  updated_at timestamp not null default current_timestamp
);

create trigger agent_identities_updated_at 
after update on agent_identities
for each row
begin
    update agent_identities set updated_at = current_timestamp where id = old.id;
end;

create index idx_agent_identities_fingerprint on agent_identities(pubkey_fingerprint);
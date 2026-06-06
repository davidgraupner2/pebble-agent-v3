Create table agent_jwt (
  id integer not null primary key autoincrement,
  registration_id text not null,
  jti text not null,
  status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'expired'))
);

create unique index idx_agent_jwt on agent_jwt(registration_id,jti);


create table subscriptions(
    id uuid primary key default uuidv7(),
    email text not null unique,
    name text not null,
    subscribed_at timestamptz not null default now()
);
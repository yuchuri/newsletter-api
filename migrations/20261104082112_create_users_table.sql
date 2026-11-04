create table users (
    user_id uuid primary key default uuidv7(),
    username text not null unique,
    password text not null
);
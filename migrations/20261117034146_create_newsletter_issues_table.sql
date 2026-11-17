create table newsletter_issues (
    newsletter_issue_id uuid primary key default uuidv7(),
    title text not null,
    text_content text not null,
    html_content text not null,
    published_at timestamptz not null default now()
);
use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use tracing::{Span, field};
use uuid::Uuid;

use crate::{configuration::Settings, domain::SubscriberEmail, state::EmailClient};

pub async fn run_worker_until_stopped(configuration: Settings) -> anyhow::Result<()> {
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.connection_options());

    let email_client = configuration.email_client.client();
    worker_loop(connection_pool, email_client).await
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = tracing::field::Empty,
        subscriber_email = tracing::field::Empty,
    )
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionOutcome> {
    if let Some((transaction, issue_id, email)) = dequeue_task(pool).await? {
        Span::current().record("newsletter_issue_id", field::display(&issue_id));
        Span::current().record("subscriber_email", field::display(&email));
        match SubscriberEmail::try_from(email.clone()) {
            Ok(email) => {
                let issue = get_issue(pool, issue_id).await?;
                if let Err(err) = email_client
                    .send_email(
                        &email,
                        &issue.title,
                        &issue.html_content,
                        &issue.text_content,
                    )
                    .await
                {
                    tracing::error!(
                        error.cause_chain = ?err,
                        error.message = %err,
                        "Failed to deliver issue to a confirmed subscriber. \
                        Skipping"
                    );
                }
            }
            Err(err) => {
                tracing::error!(
                    error.cause_chain = ?err,
                    error.message = %err,
                    "Failed to parse subscriber email address. Skipping",
                );
            }
        }
        delete_task(transaction, issue_id, &email).await?;
        Ok(ExecutionOutcome::TaskCompleted)
    } else {
        Ok(ExecutionOutcome::EmptyQueue)
    }
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(pool: &PgPool) -> anyhow::Result<Option<(PgTransaction, Uuid, String)>> {
    let mut transaction = pool.begin().await?;
    let row = sqlx::query!(
        r#"
        select newsletter_issue_id, subscriber_email
        from issue_delivery_queue
        for update skip locked
        limit 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(row) = row {
        Ok(Some((
            transaction,
            row.newsletter_issue_id,
            row.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    newsletter_issue_id: Uuid,
    subscriber_email: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        delete from issue_delivery_queue
        where 
            newsletter_issue_id = $1 and
            subscriber_email = $2
        "#,
        newsletter_issue_id,
        subscriber_email
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await.map_err(anyhow::Error::from)
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(pool: &PgPool, issue_id: Uuid) -> anyhow::Result<NewsletterIssue> {
    sqlx::query_as!(
        NewsletterIssue,
        r#"
        select title, text_content, html_content
        from newsletter_issues
        where
            newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(pool)
    .await
    .map_err(anyhow::Error::from)
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> anyhow::Result<()> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

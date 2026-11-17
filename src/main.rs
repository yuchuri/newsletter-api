use std::fmt::{Debug, Display};

use newsletter_api::{
    configuration, issue_delivery_worker::run_worker_until_stopped, startup, telemetry,
};
use tokio::task::JoinError;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let configuration = configuration::get().expect("Failed to read configuration.");

    let application = tokio::task::spawn(startup::build(configuration.clone()));
    let worker = tokio::spawn(run_worker_until_stopped(configuration));

    tokio::select! {
        o = application => report_exit("API", o),
        o = worker => report_exit("Background worker", o)
    };
    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{task_name} has exited.");
        }
        Ok(Err(err)) => {
            tracing::error!(
                error.cause_chain = ?err,
                error.message = %err,
                "{task_name} failed."
            )
        }
        Err(err) => {
            tracing::error!(
                error.cause_chain = ?err,
                error.message = %err,
                "{task_name}'s task failed to complete."
            )
        }
    }
}

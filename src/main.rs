use email_newsletter::configuration::get_configuration;
use email_newsletter::idempotency_cleanup::run_cleanup_worker;
use email_newsletter::issue_delivery_queue::run_worker_until_stopped;
use email_newsletter::startup::Application;
use email_newsletter::telemetry::{get_subscriber, init_subscriber};
use tokio::task::JoinError;
use std::fmt::{Debug, Display};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("email_newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let application = Application::build(configuration.clone()).await?;
    let application_task = tokio::spawn(application.run_until_stopped());

    let worker_task = tokio::spawn(run_worker_until_stopped(configuration.clone()));

    let cleanup_task = tokio::spawn(run_cleanup_worker(configuration));

    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
        o = cleanup_task => report_exit("Idempotency cleanup worker", o),
    };

    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name,
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "'{}' tasks failed to complete",
                task_name,
            )
        }
    }
}
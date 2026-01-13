use axum::routing::{get, post};
use axum::{middleware, serve::Serve, Router};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::Key;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions::service::PrivateCookie;
use tower_sessions_redis_store::{
    fred::{
        interfaces::ClientLike,
        prelude::{Config, Pool},
    },
    RedisStore,
};

use crate::authentication::AuthenticatedUser;
use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, home, log_out,
    login, login_form, newsletters_form, publish_newsletter, subscribe,
};

pub struct Application {
    port: u16,
    server: Serve<TcpListener, Router, Router>,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let email_client = configuration.email_client.client();
        let redis_config = Config::from_url(configuration.redis_uri.expose_secret().as_str())?;
        // Use a smaller pool size to avoid connection issues
        let pool_size = if cfg!(test) { 1 } else { 6 };
        let redis_pool = Pool::new(redis_config, None, None, None, pool_size)?;

        // Connect to Redis with a timeout
        let connect_future = async {
            let _handles = redis_pool.connect();
            redis_pool.wait_for_connect().await
        };

        tokio::time::timeout(std::time::Duration::from_secs(5), connect_future)
            .await
            .map_err(|_| anyhow::anyhow!("Redis connection timeout"))??;

        let redis_store = RedisStore::new(redis_pool.clone());
        let key = Key::derive_from(
            configuration
                .application
                .hmac_secret
                .expose_secret()
                .as_bytes(),
        );
        let session_layer = SessionManagerLayer::new(redis_store)
            .with_private(key)
            .with_expiry(Expiry::OnInactivity(Duration::seconds(60 * 60 * 24 * 30)));

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address).await?;

        let port = listener.local_addr().unwrap().port();

        let state = AppState {
            db_pool: connection_pool.clone(),
            email_client: email_client.clone(),
            base_url: ApplicationBaseUrl(configuration.application.base_url.clone()),
        };

        let server = run(listener, state, session_layer)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub async fn run_with_graceful_shutdown(
        self,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), std::io::Error> {
        self.server
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: ApplicationBaseUrl,
}

impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db_pool.clone()
    }
}

impl axum::extract::FromRef<AppState> for EmailClient {
    fn from_ref(state: &AppState) -> Self {
        state.email_client.clone()
    }
}

impl axum::extract::FromRef<AppState> for ApplicationBaseUrl {
    fn from_ref(state: &AppState) -> Self {
        state.base_url.clone()
    }
}

fn build_router(
    session_layer: SessionManagerLayer<RedisStore<Pool>, PrivateCookie>,
) -> Router<AppState> {
    let admin_routes = Router::<AppState>::new()
        .route("/dashboard", get(admin_dashboard))
        .route(
            "/newsletters",
            get(newsletters_form).post(publish_newsletter),
        )
        .route("/password", get(change_password_form).post(change_password))
        .route("/logout", post(log_out))
        .route_layer(middleware::from_extractor::<AuthenticatedUser>());

    Router::<AppState>::new()
        .route("/", get(home))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/login", get(login_form).post(login))
        .nest("/admin", admin_routes)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
}

fn run(
    listener: TcpListener,
    state: AppState,
    session_layer: SessionManagerLayer<RedisStore<Pool>, PrivateCookie>,
) -> Result<axum::serve::Serve<TcpListener, Router, Router>, anyhow::Error> {
    let app: Router = build_router(session_layer).with_state::<()>(state);
    let server = axum::serve(listener, app);

    Ok(server)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

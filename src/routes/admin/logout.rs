use axum::response::Redirect;

use crate::{session_state::TypedSession, utils::e500};

pub async fn log_out(session: TypedSession) -> Result<Redirect, crate::utils::AppError> {
    if session.get_user_id().await.map_err(e500)?.is_some() {
        session.log_out().await.map_err(e500)?;
        session.flash_info("You have successfully logged out").await;
    }

    Ok(Redirect::to("/login"))
}

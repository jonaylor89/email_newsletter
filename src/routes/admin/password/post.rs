use axum::extract::{Form, State};
use axum::response::Redirect;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, AuthenticatedUser, Credentials},
    domain::Password,
    routes::get_username,
    session_state::TypedSession,
    utils::e500,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    AuthenticatedUser(user_id): AuthenticatedUser,
    State(pool): State<PgPool>,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<Redirect, crate::utils::AppError> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        session
            .flash_error("You entered two different new passwords - the field values must match")
            .await;
        return Ok(Redirect::to("/admin/password"));
    }

    let new_password: Result<Password, _> = form.new_password.expose_secret().try_into();

    if new_password.is_err() {
        session
            .flash_error("You entered an invalid new password")
            .await;
        return Ok(Redirect::to("/admin/password"));
    }

    let new_password = new_password.unwrap();

    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                session
                    .flash_error("The current password is incorrect")
                    .await;
                Ok(Redirect::to("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(*user_id, new_password, &pool)
        .await
        .map_err(e500)?;

    session.flash_info("Your password has been changed").await;
    Ok(Redirect::to("/admin/password"))
}

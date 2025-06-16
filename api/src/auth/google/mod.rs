mod client;

use std::collections::HashMap;

use crate::{error::AppResult, sqlite::Database, AppState};
use axum::{
    debug_handler,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use oauth2::{AuthorizationCode, CsrfToken};

pub use client::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    return_path: String,
}

#[debug_handler(state = AppState)]
pub async fn login(
    State(client): State<Client>,
    State(db): State<Database>,
    Query(params): Query<LoginParams>,
) -> AppResult<Redirect> {
    let authorize_url = client
        .authorize_url(db.as_ref(), &params.return_path)
        .await?;

    Ok(Redirect::to(&authorize_url))
}

#[debug_handler(state = AppState)]
pub async fn callback(
    State(client): State<Client>,
    Query(mut params): Query<HashMap<String, String>>,
    State(db): State<Database>,
) -> AppResult<impl IntoResponse> {
    let state = CsrfToken::new(
        params
            .remove("state")
            .ok_or(anyhow::anyhow!("OAuth: without state"))?,
    );
    let code = AuthorizationCode::new(
        params
            .remove("code")
            .ok_or(anyhow::anyhow!("OAuth: without code"))?,
    );

    let (session_token, redirect_url) = client.callback(code, state, db.as_ref()).await?;

    let headers = axum::response::AppendHeaders([(
        axum::http::header::SET_COOKIE,
        format!(
            "session_token={}; path=/; httponly; secure; samesite=strict",
            session_token
        ),
    )]);

    tracing::warn!("redirect_url: {}", redirect_url);

    Ok((headers, Redirect::to(&redirect_url)))
}

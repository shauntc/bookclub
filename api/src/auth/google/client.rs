use anyhow::{Context, Result};

use oauth2::{EndpointMaybeSet, EndpointNotSet, EndpointSet};
use openidconnect::core::{
    CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod, CoreGrantType,
    CoreIdTokenClaims, CoreIdTokenVerifier, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm, CoreResponseMode, CoreResponseType, CoreRevocableToken,
    CoreSubjectIdentifierType,
};
use openidconnect::{
    AdditionalProviderMetadata, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse, ProviderMetadata, RedirectUrl, RevocationUrl,
    Scope,
};
use serde::{Deserialize, Serialize};
use sqlx::Pool;
use uuid::Uuid;

use crate::users::User;

// Teach openidconnect-rs about a Google custom extension to the OpenID Discovery response that we can use as the RFC
// 7009 OAuth 2.0 Token Revocation endpoint. For more information about the Google specific Discovery response see the
// Google OpenID Connect service documentation at: https://developers.google.com/identity/protocols/oauth2/openid-connect#discovery
#[derive(Clone, Debug, Deserialize, Serialize)]
struct RevocationEndpointProviderMetadata {
    revocation_endpoint: String,
}
impl AdditionalProviderMetadata for RevocationEndpointProviderMetadata {}
type GoogleProviderMetadata = ProviderMetadata<
    RevocationEndpointProviderMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

type GoogleClient = CoreClient<
    EndpointSet,      // HasAuthUrl
    EndpointNotSet,   // HasDeviceAuthUrl
    EndpointNotSet,   // HasIntrospectionUrl
    EndpointSet,      // HasRevocationUrl
    EndpointMaybeSet, // HasTokenUrl
    EndpointMaybeSet, // HasUserInfoUrl
>;

#[derive(Debug, Deserialize)]
pub struct Settings {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    verified_email: bool,
    given_name: String,
    family_name: String,
}

#[derive(Clone)]
pub struct Client {
    client: GoogleClient,
    http_client: openidconnect::reqwest::Client,
}

impl Client {
    pub async fn new(host_url: String, settings: Settings) -> Result<Self> {
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())?;
        let redirect_url = format!("{}/auth/google/callback", host_url);

        let http_client = openidconnect::reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .build()?;

        let provider_metadata =
            GoogleProviderMetadata::discover_async(issuer_url, &http_client).await?;

        let revocation_endpoint = provider_metadata
            .additional_metadata()
            .revocation_endpoint
            .clone();

        let google_client_id = ClientId::new(settings.client_id);
        let google_client_secret = ClientSecret::new(settings.client_secret);

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            google_client_id,
            Some(google_client_secret),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url)?)
        .set_revocation_url(RevocationUrl::new(revocation_endpoint)?);

        Ok(Self {
            client,
            http_client,
        })
    }

    pub async fn authorize_url(
        &self,
        db_pool: &Pool<sqlx::Sqlite>,
        return_url: &str,
    ) -> Result<String> {
        let (authorize_url, csrf_state, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        sqlx::query(
            "INSERT INTO oauth2_state_storage (csrf_state, nonce, return_url) VALUES (?, ?, ?);",
        )
        .bind(csrf_state.secret())
        .bind(nonce.secret())
        .bind(return_url)
        .execute(db_pool)
        .await?;

        Ok(authorize_url.to_string())
    }

    pub async fn callback(
        &self,
        code: AuthorizationCode,
        state: CsrfToken,
        db_pool: &Pool<sqlx::Sqlite>,
    ) -> Result<(String, String)> {
        let (nonce, return_url): (String, String) = sqlx::query_as(
            r#"DELETE FROM oauth2_state_storage WHERE csrf_state = ? RETURNING nonce, return_url"#,
        )
        .bind(state.secret())
        .fetch_one(db_pool)
        .await?;

        let nonce = Nonce::new(nonce);

        let token_response = self
            .client
            .exchange_code(code)?
            .request_async(&self.http_client)
            .await?;
        let access_token = token_response.access_token().secret();

        let id_token_verifier: CoreIdTokenVerifier = self.client.id_token_verifier();
        let id_token_claims: &CoreIdTokenClaims = token_response
            .extra_fields()
            .id_token()
            .expect("Server did not return an ID token")
            .claims(&id_token_verifier, &nonce)?;

        tracing::warn!("Google returned ID token: {:?}", id_token_claims);

        let url =
            "https://www.googleapis.com/oauth2/v2/userinfo?oauth_token=".to_owned() + access_token;
        let res = self
            .http_client
            .get(url)
            .send()
            .await
            .context("OAuth: reqwest failed to query userinfo")?;

        let user_info: GoogleUserInfo = serde_json::from_str(
            &res.text()
                .await
                .context("OAuth: reqwest received invalid userinfo")?,
        )?;

        if !user_info.verified_email {
            return Err(anyhow::anyhow!("OAuth: email address is not verified"));
        }

        // Check if user exists in database
        // If not, create a new user
        let user: User = match sqlx::query_as(r#"SELECT * FROM users WHERE email=?"#)
            .bind(&user_info.email)
            .fetch_optional(db_pool)
            .await?
        {
            Some(user) => user,
            None => {
                sqlx::query_as(
                    r#"
                    INSERT INTO users (email, first_name, last_name)
                    VALUES (?, ?, ?)
                    RETURNING *
                    "#,
                )
                .bind(&user_info.email)
                .bind(&user_info.given_name)
                .bind(&user_info.family_name)
                .fetch_one(db_pool)
                .await?
            }
        };

        // Create a session for the user
        let session_token_p1 = Uuid::new_v4().to_string();
        let session_token_p2 = Uuid::new_v4().to_string();
        let session_token = [session_token_p1.as_str(), "_", session_token_p2.as_str()].concat();

        let created_at = chrono::Utc::now().timestamp();
        let expires_at = created_at + 60 * 60 * 24;

        sqlx::query(
            "INSERT INTO user_sessions
            (session_token_p1, session_token_p2, user_id, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?);",
        )
        .bind(session_token_p1)
        .bind(session_token_p2)
        .bind(user.id)
        .bind(created_at)
        .bind(expires_at)
        .execute(db_pool)
        .await?;

        Ok((session_token, return_url))
    }
}

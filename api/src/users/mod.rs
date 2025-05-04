mod user;

use serde::{Deserialize, Serialize};
pub use user::*;

use crate::error::AppResult;
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::Row;

use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserParams {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn create_user(
    State(state): State<AppState>,
    Json(params): Json<CreateUserParams>,
) -> AppResult<impl IntoResponse> {
    let id: i64 = sqlx::query!(
        r#"
        INSERT INTO users (email, first_name, last_name)
        VALUES (?, ?, ?)
        RETURNING id
        "#,
        params.email,
        params.first_name,
        params.last_name
    )
    .fetch_one(&state.db)
    .await?
    .id;

    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, first_name, last_name, 
               created_at, updated_at
        FROM users WHERE id = ?
        "#,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn get_users(State(state): State<AppState>) -> AppResult<Json<Vec<User>>> {
    let users = sqlx::query(
        r#"
        SELECT id, email, first_name, last_name, 
               created_at, updated_at
        FROM users
        ORDER BY id
        "#,
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|row| User {
        id: row.get("id"),
        email: row.get("email"),
        first_name: row.get("first_name"),
        last_name: row.get("last_name"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
    .collect::<Vec<_>>();

    Ok(Json(users))
}

#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, first_name, last_name, 
               created_at, updated_at
        FROM users WHERE id = ?
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    match user {
        Some(user) => Ok(Json(user).into_response()),
        None => Ok((StatusCode::NOT_FOUND, "User not found").into_response()),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserParams {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(params): Json<UpdateUserParams>,
) -> AppResult<impl IntoResponse> {
    let mut query = sqlx::QueryBuilder::new(
        r#"
        UPDATE users SET 
        "#,
    );
    let mut separated = query.separated(", ");
    if let Some(email) = params.email {
        separated.push("email = ");
        separated.push_bind_unseparated(email);
    }
    if let Some(first_name) = params.first_name {
        separated.push("first_name = ");
        separated.push_bind_unseparated(first_name);
    }
    if let Some(last_name) = params.last_name {
        separated.push("last_name = ");
        separated.push_bind_unseparated(last_name);
    }
    query.push(" WHERE id = ");
    query.push_bind(id);
    tracing::debug!("Query: {}", query.sql());
    let query = query.build();
    query.execute(&state.db).await?;

    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, first_name, last_name, 
               created_at, updated_at
        FROM users WHERE id = ?
        "#,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    match result.rows_affected() {
        0 => Ok((StatusCode::NOT_FOUND, "User not found")),
        1 => Ok((StatusCode::NO_CONTENT, "User successfully deleted")),
        _ => Ok((StatusCode::INTERNAL_SERVER_ERROR, "Multiple users deleted")),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FindUserParams {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
#[debug_handler]
#[tracing::instrument(skip(state))]
pub async fn find_users(
    Query(params): Query<FindUserParams>,
    State(state): State<AppState>,
) -> Response {
    if params.email.is_none() && params.first_name.is_none() && params.last_name.is_none() {
        return (StatusCode::BAD_REQUEST, "No search parameters provided").into_response();
    }
    let mut query = sqlx::QueryBuilder::new(
        r#"
        SELECT id, email, first_name, last_name, 
               created_at, updated_at
        FROM users WHERE 
        "#,
    );
    let mut separated = query.separated(" AND ");
    if let Some(email) = params.email {
        separated.push("email = ");
        separated.push_bind_unseparated(email);
    }
    if let Some(first_name) = params.first_name {
        separated.push("first_name = ");
        separated.push_bind_unseparated(first_name);
    }
    if let Some(last_name) = params.last_name {
        separated.push("last_name = ");
        separated.push_bind_unseparated(last_name);
    }

    tracing::debug!("Query: {}", query.sql());

    let query = query.build();
    let db_result = query.fetch_all(&state.db).await;

    match db_result {
        Ok(rows) => {
            let users = rows
                .into_iter()
                .map(|row| User {
                    id: row.get("id"),
                    email: row.get("email"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
                .collect::<Vec<_>>();

            if users.is_empty() {
                (StatusCode::NOT_FOUND, "No users found").into_response()
            } else {
                (StatusCode::OK, Json(users)).into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error fetching users: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error fetching users").into_response()
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::tests::create_test_server;
    use axum_test::TestServer;
    use tracing_test::traced_test;

    pub async fn create_user(server: &TestServer, user: CreateUserParams) -> User {
        let response = server.post("/users/create").json(&user).await;
        let user: User = response.json();
        assert!(response.status_code() == 200);
        user
    }
    pub async fn create_test_user(server: &TestServer) -> User {
        create_user(
            server,
            CreateUserParams {
                email: "test@example.com".to_string(),
                first_name: "Test".to_string(),
                last_name: "User".to_string(),
            },
        )
        .await
    }

    #[tokio::test]
    #[traced_test]
    async fn test_create_user() {
        let server = create_test_server().await;
        let created_user = create_test_user(&server).await;

        let response = server.get(&format!("/users/{}", created_user.id)).await;
        response.assert_status(StatusCode::OK);
        let user: User = response.json();
        assert_eq!(user.email, created_user.email);
        assert_eq!(user.first_name, created_user.first_name);
        assert_eq!(user.last_name, created_user.last_name);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_users() {
        let server = create_test_server().await;

        let user = create_test_user(&server).await;

        // Then get all users
        let response = server.get("/users/list").await;
        assert_eq!(response.status_code(), 200);
        let users: Vec<User> = response.json();
        assert!(!users.is_empty());
        assert_eq!(users[0].email, user.email);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_update_user() {
        let server = create_test_server().await;

        let user = create_test_user(&server).await;
        let id = user.id;

        // Then update the user
        let response = server
            .put(&format!("/users/{}", id))
            .json(&UpdateUserParams {
                email: Some("updated@example.com".to_string()),
                first_name: Some("Updated".to_string()),
                last_name: Some("Name".to_string()),
            })
            .await;

        assert_eq!(response.status_code(), 200);
        let updated_user: User = response.json();
        assert_eq!(updated_user.email, "updated@example.com");
        assert_eq!(updated_user.first_name, "Updated");
        assert_eq!(updated_user.last_name, "Name");
    }

    #[tokio::test]
    #[traced_test]
    async fn test_delete_user() {
        let server = create_test_server().await;

        let user = create_test_user(&server).await;
        let id = user.id;

        // Then delete the user
        let response = server.delete(&format!("/users/{}", id)).await;
        response.assert_status(StatusCode::NO_CONTENT);

        // Verify the user is deleted
        let response = server.get(&format!("/users/{}", id)).await;
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_users_by_email() {
        let server = create_test_server().await;
        let user = create_test_user(&server).await;

        let response = server
            .get("/users/search")
            .add_query_param("email", &user.email)
            .await;
        response.assert_status(StatusCode::OK);
        let users: Vec<User> = response.json();
        assert!(!users.is_empty());
        assert_eq!(&users[0].email, &user.email);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_users_by_first_name() {
        let server = create_test_server().await;
        let user = create_test_user(&server).await;

        let response = server
            .get("/users/search")
            .add_query_param("first_name", &user.first_name)
            .await;
        response.assert_status(StatusCode::OK);
        let users: Vec<User> = response.json();
        assert!(!users.is_empty());
        assert_eq!(&users[0].first_name, &user.first_name);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_users_by_last_name() {
        let server = create_test_server().await;
        let user = create_test_user(&server).await;

        let response = server
            .get("/users/search")
            .add_query_param("last_name", &user.last_name)
            .await;
        response.assert_status(StatusCode::OK);
        let users: Vec<User> = response.json();
        assert!(!users.is_empty());
        assert_eq!(&users[0].last_name, &user.last_name);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_users_not_found() {
        let server = create_test_server().await;
        let response = server
            .get("/users/search")
            .add_query_param("email", "nonexistent@example.com")
            .await;
        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_user_by_first_and_last_name() {
        let server = create_test_server().await;
        let user = create_test_user(&server).await;
        let _ = create_user(
            &server,
            CreateUserParams {
                email: "test2@example.com".to_string(),
                first_name: format!("{}2", user.first_name),
                last_name: user.last_name.clone(),
            },
        )
        .await;
        let _ = create_user(
            &server,
            CreateUserParams {
                email: "test3@example.com".to_string(),
                first_name: user.first_name.clone(),
                last_name: format!("{}2", user.last_name),
            },
        )
        .await;

        let response = server
            .get(&format!(
                "/users/search?first_name={}&last_name={}",
                user.first_name, user.last_name
            ))
            .await;

        response.assert_status(StatusCode::OK);
        let users: Vec<User> = response.json();
        assert_eq!(users.len(), 1);
        assert_eq!(&users[0].email, &user.email);
        assert_eq!(&users[0].first_name, &user.first_name);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_find_user_by_email_and_first_name_not_found() {
        let server = create_test_server().await;
        let user = create_test_user(&server).await;

        let response = server
            .get(&format!(
                "/users/search?email={}&first_name={}",
                user.email, "nonexistent"
            ))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }
}

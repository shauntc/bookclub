mod club;
pub mod memberships;

pub use club::*;
use sqlx::Row;

use crate::error::AppResult;
use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::sqlite::Database;

#[derive(Deserialize, Serialize)]
pub struct CreateClubParams {
    name: String,
    description: String,
}

#[debug_handler]
pub async fn create_club(
    State(db): State<Database>,
    Json(CreateClubParams { name, description }): Json<CreateClubParams>,
) -> AppResult<impl IntoResponse> {
    let now = Utc::now().naive_utc();
    let id = sqlx::query!(
        r#"
        INSERT INTO clubs (name, description, created_at, updated_at)
        VALUES (?, ?, ?, ?)
        RETURNING id
        "#,
        name,
        description,
        now,
        now
    )
    .fetch_one(db.as_ref())
    .await?
    .id;

    let club = sqlx::query_as!(
        Club,
        r#"
        SELECT id, name, description, created_at, updated_at
        FROM clubs WHERE id = ?
        "#,
        id
    )
    .fetch_one(db.as_ref())
    .await?;

    Ok((StatusCode::CREATED, Json(club)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct UpdateClubParams {
    name: Option<String>,
    description: Option<String>,
}

#[debug_handler]
pub async fn update_club(
    State(db): State<Database>,
    Path(id): Path<i64>,
    Json(params): Json<UpdateClubParams>,
) -> AppResult<Json<Club>> {
    let now = Utc::now().naive_utc();

    let mut query = sqlx::QueryBuilder::new(
        r#"
        UPDATE clubs SET 
        "#,
    );
    let mut separated = query.separated(", ");
    if let Some(name) = params.name {
        separated.push("name = ");
        separated.push_bind_unseparated(name);
    }
    if let Some(description) = params.description {
        separated.push("description = ");
        separated.push_bind_unseparated(description);
    }
    separated.push("updated_at = ");
    separated.push_bind_unseparated(now);
    query.push(" WHERE id = ");
    query.push_bind(id);
    tracing::debug!("Query: {}", query.sql());
    let query = query.build();
    query.execute(db.as_ref()).await?;

    let club = sqlx::query_as!(
        Club,
        r#"
        SELECT id, name, description, created_at, updated_at
        FROM clubs WHERE id = ?
        "#,
        id
    )
    .fetch_one(db.as_ref())
    .await?;

    Ok(Json(club))
}

#[debug_handler]
pub async fn get_clubs(State(db): State<Database>) -> AppResult<Json<Vec<Club>>> {
    let clubs = sqlx::query(
        r#"
        SELECT id, name, description, created_at, updated_at
        FROM clubs
        ORDER BY id
        "#,
    )
    .fetch_all(db.as_ref())
    .await?
    .into_iter()
    .map(|row| Club {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
    .collect::<Vec<_>>();

    Ok(Json(clubs))
}

#[debug_handler]
pub async fn get_club_by_id(
    State(db): State<Database>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let club = sqlx::query_as!(
        Club,
        "SELECT id, name, description, created_at, updated_at FROM clubs WHERE id = ?",
        id
    )
    .fetch_optional(db.as_ref())
    .await?;

    match club {
        Some(club) => Ok(Json(club).into_response()),
        None => Ok((StatusCode::NOT_FOUND, "Club not found").into_response()),
    }
}

#[debug_handler]
pub async fn delete_club(
    State(db): State<Database>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    sqlx::query!("DELETE FROM clubs WHERE id = ?", id)
        .execute(db.as_ref())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::tests::create_test_server;
    use axum_test::TestServer;

    pub async fn create_club(server: &TestServer, club: CreateClubParams) -> Club {
        let response = server.post("/clubs").json(&club).await;
        response.assert_status(StatusCode::CREATED);
        response.json()
    }

    pub async fn create_test_club(server: &TestServer) -> Club {
        create_club(
            server,
            CreateClubParams {
                name: "Test Club".to_string(),
                description: "Test Description".to_string(),
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_create_club() {
        let server = create_test_server().await;
        let club = create_club(
            &server,
            CreateClubParams {
                name: "Test Club".to_string(),
                description: "Test Description".to_string(),
            },
        )
        .await;

        assert_eq!(club.name, "Test Club");
        assert_eq!(club.description, "Test Description");
    }

    #[tokio::test]
    async fn test_get_clubs() {
        let server = create_test_server().await;
        let club = create_test_club(&server).await;

        // Then get all clubs
        let response = server.get("/clubs/list").await;
        assert_eq!(response.status_code(), 200);
        let clubs: Vec<Club> = response.json();
        assert!(!clubs.is_empty());
        assert_eq!(clubs[0].name, club.name);
    }

    #[tokio::test]
    async fn test_update_club() {
        let server = create_test_server().await;
        let club = create_test_club(&server).await;
        let id = club.id;

        // Then update it
        let response = server
            .put(&format!("/clubs/{}", id))
            .json(&UpdateClubParams {
                name: Some("Updated Club".to_string()),
                description: Some("Updated Description".to_string()),
            })
            .await;

        assert_eq!(response.status_code(), 200);
        let updated_club: Club = response.json();
        assert_eq!(updated_club.name, "Updated Club");
        assert_eq!(updated_club.description, "Updated Description");
    }

    #[tokio::test]
    async fn test_delete_club() {
        let server = create_test_server().await;
        let club = create_test_club(&server).await;
        let id = club.id;

        // Then delete it
        let response = server.delete(&format!("/clubs/{}", id)).await;
        assert_eq!(response.status_code(), 204);

        // Verify it's deleted
        let response = server.get(&format!("/clubs/{}", id)).await;
        assert_eq!(response.status_code(), 404);
    }
}

mod membership;

pub use membership::*;

use crate::error::AppResult;
use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize, Serialize)]
pub struct CreateMembershipParams {
    user_id: i64,
    club_id: i64,
    permission_level: i32,
}

#[debug_handler]
pub async fn create_membership(
    State(state): State<AppState>,
    Json(CreateMembershipParams {
        user_id,
        club_id,
        permission_level,
    }): Json<CreateMembershipParams>,
) -> AppResult<impl IntoResponse> {
    // Validate permission level
    if permission_level < 0 || permission_level > 2 {
        return Ok((
            StatusCode::BAD_REQUEST,
            "Permission level must be between 0 and 2",
        )
            .into_response());
    }

    let id = sqlx::query!(
        r#"
        INSERT INTO memberships (user_id, club_id, permission_level)
        VALUES (?, ?, ?)
        RETURNING id
        "#,
        user_id,
        club_id,
        permission_level
    )
    .fetch_one(&state.db)
    .await?
    .id;

    let membership = sqlx::query_as!(
        Membership,
        r#"
        SELECT id, user_id, club_id, permission_level, created_at
        FROM memberships
        WHERE id = ?
        "#,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(membership)).into_response())
}

#[debug_handler]
pub async fn delete_membership(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let result = sqlx::query!(
        r#"
        DELETE FROM memberships
        WHERE id = ?
        "#,
        id
    )
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Ok((StatusCode::NOT_FOUND, "Membership not found").into_response());
    }

    Ok((StatusCode::OK, "Membership deleted successfully").into_response())
}

#[debug_handler]
pub async fn get_memberships(State(state): State<AppState>) -> AppResult<Json<Vec<Membership>>> {
    let memberships = sqlx::query_as!(
        Membership,
        r#"
        SELECT id, user_id, club_id, permission_level, created_at
        FROM memberships
        ORDER BY id
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(memberships))
}

#[debug_handler]
pub async fn get_membership_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let membership = sqlx::query_as!(
        Membership,
        r#"
        SELECT id, user_id, club_id, permission_level, created_at
        FROM memberships
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    match membership {
        Some(m) => Ok(Json(m).into_response()),
        None => Ok((StatusCode::NOT_FOUND, "Membership not found").into_response()),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::clubs::test::create_test_club;
    use crate::tests::create_test_server;
    use crate::users::test::create_test_user;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_create_membership() {
        let server = create_test_server().await;

        let user = crate::users::test::create_test_user(&server).await;

        let club = create_test_club(&server).await;

        // Create membership
        let response = server
            .post("/memberships")
            .json(&CreateMembershipParams {
                user_id: user.id,
                club_id: club.id,
                permission_level: 1,
            })
            .await;

        response.assert_status(StatusCode::CREATED);
        let membership: Membership = response.json();
        assert_eq!(membership.user_id, user.id);
        assert_eq!(membership.club_id, club.id);
        assert_eq!(membership.permission_level, 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_delete_membership() {
        let server = create_test_server().await;

        let user = create_test_user(&server).await;

        let club = create_test_club(&server).await;

        let membership_response = server
            .post("/memberships")
            .json(&CreateMembershipParams {
                user_id: user.id,
                club_id: club.id,
                permission_level: 1,
            })
            .await;
        let membership: Membership = membership_response.json();

        // Delete the membership
        let response = server
            .delete(&format!("/memberships/{}", membership.id))
            .await;
        assert_eq!(response.status_code(), 200);

        // Verify it's deleted
        let response = server.get(&format!("/memberships/{}", membership.id)).await;
        assert_eq!(response.status_code(), 404);
    }
}

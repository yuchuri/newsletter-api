use axum::{extract::FromRequestParts, http::request::Parts};
use tower_sessions::Session;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &str = "user_id";

    pub async fn cycle_id(&self) -> anyhow::Result<()> {
        self.0.cycle_id().await?;
        Ok(())
    }

    pub async fn insert_user_id(&self, user_id: Uuid) -> anyhow::Result<()> {
        self.0.insert(Self::USER_ID_KEY, user_id).await?;
        Ok(())
    }

    pub async fn get_user_id(&self) -> anyhow::Result<Option<Uuid>> {
        let user_id = self.0.get::<Uuid>(Self::USER_ID_KEY).await?;
        Ok(user_id)
    }

    pub async fn logout(&self) -> anyhow::Result<()> {
        self.0.flush().await?;
        Ok(())
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = <Session as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        Ok(TypedSession(session))
    }
}

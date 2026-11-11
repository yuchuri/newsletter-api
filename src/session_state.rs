use axum::{extract::FromRequestParts, http::request::Parts};
use tower_sessions::{Session, session};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &str = "user_id";

    pub async fn cycle_id(&self) -> Result<(), session::Error> {
        self.0.cycle_id().await
    }

    pub async fn insert_user_id(&self, user_id: Uuid) -> Result<(), session::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>, session::Error> {
        self.0.get::<Uuid>(Self::USER_ID_KEY).await
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = <Session as FromRequestParts<S>>::Rejection;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async {
            let session = Session::from_request_parts(parts, state).await?;
            Ok(TypedSession(session))
        }
    }
}

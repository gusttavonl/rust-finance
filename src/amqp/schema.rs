use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PaymentMessage {
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub userId: Uuid,
    pub categoryId: Uuid,
}

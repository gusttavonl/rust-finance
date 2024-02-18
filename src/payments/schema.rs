use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct ParamOptions {
    pub id: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePaymentSchema {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub userId: Uuid,
    pub categoryId: Uuid,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePaymentSchema {
    pub name: String,
    pub price: f64,
    pub description: String,
}
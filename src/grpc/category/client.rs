use category::category_client::CategoryClient;
use category::GetCategoryRequest;
use uuid::Uuid;

pub mod category {
    tonic::include_proto!("category");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CategoryClient::connect("http://[::1]:50051").await?;

    let category_id = Uuid::parse_str("any_id")?;

    let request = tonic::Request::new(GetCategoryRequest {
        id: category_id.to_string(),
    });

    println!("Sending request to gRPC Server...");
    let response = client.get_category(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}

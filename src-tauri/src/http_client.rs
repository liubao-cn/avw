use reqwest::Client;

pub fn create_http_client() -> Result<Client, String> {
    Client::builder()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}


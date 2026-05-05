use dotenvy::dotenv;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
struct Auth0TokenRequest {
    client_id: String,
    client_secret: String,
    audience: String,
    grant_type: String,
}

#[derive(Deserialize, Debug)]
struct Auth0TokenResponse {
    access_token: String,
}

pub fn get_auth0_1() -> (String, String) {
    static TOKEN_01: OnceCell<(String, String)> = OnceCell::new();

    TOKEN_01
        .get_or_init(|| {
            dotenv().ok();
            let client_id = std::env::var("AUTH0_CLIENT_ID_1").expect("Missing AUTH0_CLIENT_ID_1");
            let client_secret =
                std::env::var("AUTH0_CLIENT_SECRET_1").expect("Missing AUTH0_CLIENT_SECRET_1");
            get_token(client_id, client_secret)
        })
        .clone()
}

pub fn get_auth0_2() -> (String, String) {
    static TOKEN_02: OnceCell<(String, String)> = OnceCell::new();

    TOKEN_02
        .get_or_init(|| {
            dotenv().ok();
            let client_id = std::env::var("AUTH0_CLIENT_ID_2").expect("Missing AUTH0_CLIENT_ID_2");
            let client_secret =
                std::env::var("AUTH0_CLIENT_SECRET_2").expect("Missing AUTH0_CLIENT_SECRET_2");
            get_token(client_id, client_secret)
        })
        .clone()
}

fn get_token(client_id: String, client_secret: String) -> (String, String) {
    let token_request = Auth0TokenRequest {
        client_id: client_id.clone(),
        client_secret: client_secret,
        audience: std::env::var("AUTH0_AUDIENCE").expect("Missing AUTH0_AUDIENCE"),
        grant_type: "client_credentials".to_string(),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&format!(
            "https://{}/oauth/token",
            std::env::var("AUTH0_DOMAIN").expect("Missing AUTH0_DOMAIN")
        ))
        .json(&token_request)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    let token_response: Auth0TokenResponse = response.json().expect("Failed to parse response");
    (
        token_response.access_token,
        format!("{}@clients", client_id),
    )
}

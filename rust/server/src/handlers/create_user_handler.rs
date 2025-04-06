use std::{str::FromStr, sync::Arc};

use axum::{extract::State, Json};
use sessionless::{secp256k1::PublicKey, Sessionless, Signature};

use crate::{config::AppState, storage::PubKeys};

use super::{CreateUserRequest, Response};


// Creates a new user if pubKey does not exist, and returns existing uuid if it does.
// signature message is: timestamp + pubKey + hash
pub async fn create_user_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateUserRequest>,
) -> Json<Response> { 
    let message = format!("{}{}{}", body.timestamp, body.pub_key, body.hash);
    let sessionless = Sessionless::new();

    if let Ok(pub_key) = PublicKey::from_str(body.pub_key.as_str()) {
        let sig = match Signature::from_str(body.signature.as_str()) {
            Ok(s) => s,
            Err(_) => {
                return Json(Response::auth_error());
            }
        };

        if sessionless.verify(message, &pub_key, &sig).is_err() {
            return Json(Response::auth_error());
        }

        let key = PubKeys::key(&body.hash, &body.pub_key);
        match data.user_client.clone().get_user_uuid(&key).await {
            // If user exists with given (pub_key + hash), return back the user_uuid
            Some(user_uuid) => Json(Response::user_success(user_uuid)),
            None => {
                // otherwise, put a new user
                let new_uuid = Sessionless::generate_uuid();
                match data.user_client.clone().put_user(&new_uuid.to_string(), &body.pub_key, &body.hash).await {
                    Ok(user) => {
                        // add pub key + hash with user uuid
                        match data.user_client.clone().update_keys(&key, &user.uuid).await {
                            Ok(_) => Json(Response::user_success(user.uuid)),
                            Err(_) => Json(Response::server_error("Failed to update keys".to_string()))
                        }
                    },
                    Err(_) => Json(Response::server_error("Failed to put user".to_string()))
                }
            }
        }
    } else {
        return Json(Response::auth_error());
    }
}


#[cfg(test)]
mod tests {

    use chrono::Utc;
    use sessionless::Sessionless;

    use crate::handlers::{CreateUserRequest, Response};
    use crate::storage::PubKeys;
    use crate::test_common::{self, check_path_exists, cleanup_test_files, read_keys, setup_test_server, storage_uri};

    #[tokio::test]
    async fn test_create_user_handler() {
        let storage_uri = storage_uri("test_create_user_handler");
        let test_server = setup_test_server(storage_uri.clone());

        assert!(test_server.is_running());
        let sessionless = Sessionless::new();

        let pub_key = sessionless.public_key();
        let timestamp = Utc::now().timestamp().to_string();
        let hash = "random_hash".to_string();

        let message = format!("{}{}{}", timestamp, pub_key, hash);
        let signature = sessionless.sign(message);

        let payload = CreateUserRequest {
            pub_key: pub_key.to_string(),
            timestamp: timestamp,
            hash: hash.clone(),
            signature: signature.to_string(),
        };


        let response = test_server.post(test_common::USER_CREATE_PATH).json(&payload).await;

        assert_eq!(response.clone().status_code(), 200);
        // get the user_uuid from the response
        // parse as Response
        let user_resposne = response.json::<Response>();

        match user_resposne.clone() {
            Response::User { user_uuid } => {
                assert_eq!(user_uuid.is_empty(), false);
                // check that the user file created exists
                let file_path = format!("{}/user:{}", storage_uri.to_string(), user_uuid);
                assert!(check_path_exists(file_path.as_str()).await);

                // check the keys file also exists
                let keys_file_path = format!("{}/keys", storage_uri.to_string());
                assert!(check_path_exists(keys_file_path.as_str()).await);

                // TODO check the keys file has the correct pub_key + hash  and user_uuid
                let key = PubKeys::key(&hash.clone(), &pub_key.to_string());
                let pub_keys = read_keys(&storage_uri.to_string()).await.expect("Failed to read keys");
                assert!(pub_keys.num_keys() == 1);
                assert!(pub_keys.get_user_uuid(key.as_str()).is_some());
                assert_eq!(pub_keys.get_user_uuid(key.as_str()).unwrap(), &user_uuid);
            },
            _ => {
                assert!(false);
            }
        }
        cleanup_test_files(&storage_uri.to_string()).await;
    }

    #[tokio::test]
    async fn test_create_user_handler_auth_error() {
        let storage_uri = storage_uri("test_create_user_handler_auth_error");
        let test_server = setup_test_server(storage_uri.clone());

        assert!(test_server.is_running());
        let sessionless = Sessionless::new();

        let pub_key = sessionless.public_key();
        let timestamp = Utc::now().timestamp().to_string();
        let hash = "random_hash".to_string();

        let invalid_payload = CreateUserRequest {
            pub_key: pub_key.to_string(),
            timestamp: timestamp.clone(),
            hash: hash.clone(),
            signature: "invalid_signature".to_string(),
        };

        let post_path = "/user/create";

        let response = test_server.post(post_path).json(&invalid_payload).await;

        let expected_code = 403;

        // parse as Response
        let error_response = response.json::<Response>();

        match error_response.clone() {
            Response::Error { code, message } => {
                assert_eq!(code, expected_code);
                assert_eq!(message, "Auth Error");
            },
            _ => {
                assert!(false);
            }
        }

        let message = format!("{}{}{}", &timestamp, pub_key, &hash);
        let signature = sessionless.sign(message);

        let invalid_payload = CreateUserRequest {
            pub_key: "invalid_pub_key".to_string(),
            timestamp: timestamp.clone(),
            hash: hash.clone(),
            signature: signature.to_string(),
        };

        let response = test_server.post(post_path).json(&invalid_payload).await;

        // parse as Response
        let error_response = response.json::<Response>();
        match error_response.clone() {
            Response::Error { code, message } => {
                assert_eq!(code, expected_code);
                assert_eq!(message, "Auth Error");
            },
            _ => {
                assert!(false);
            }
        }

        // TODO handle internal server errors
    }
}
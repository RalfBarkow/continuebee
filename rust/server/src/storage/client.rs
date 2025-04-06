use axum::http::Uri;

use async_trait::async_trait;
use super::{FileStorageClient, NotImplementedYetClient, StorageClient};

fn is_file_uri(uri: &Uri) -> bool {
    // if scheme is none
    uri.scheme().is_none()
}


#[derive(Debug, Clone)]
pub enum Client {
    FileStorageClient {storage_client: FileStorageClient },
    NotImplementedYet {storage_client: NotImplementedYetClient }
}

impl Client {
    pub fn new(storage_uri: Uri) -> Self {
        if is_file_uri(&storage_uri) {
            return Client::FileStorageClient { storage_client: FileStorageClient::new(storage_uri) };
        }
        Client::NotImplementedYet {storage_client: NotImplementedYetClient {}}
    }
}

#[async_trait]
impl StorageClient for Client {
    async fn get(&self, key: &str) -> Option<serde_json::Value> {
        match self {
            Client::FileStorageClient { storage_client } => storage_client.get(key).await,
            Client::NotImplementedYet { storage_client} => storage_client.get(key).await,
        }
    }
    // Set a json value in the storage; will create new file if it doesnt exist or overwrite otherwise
    async fn set(&self, key: &str, value: serde_json::Value) -> anyhow::Result<()> {
        match self {
            Client::FileStorageClient { storage_client } => storage_client.set(key, value).await,
            Client::NotImplementedYet { storage_client} => storage_client.set(key, value).await,
        }
    }
    // Delete from the storage; returns true if the value was deleted
    async fn delete(&self, key: &str) -> bool {
        match self {
            Client::FileStorageClient { storage_client } => storage_client.delete(key).await,
            Client::NotImplementedYet { storage_client} => storage_client.delete(key).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_uri() {

        let uri = Uri::from_static("http://example.com");
        assert!(!is_file_uri(&uri));

        let uri = Uri::from_static("/tmp");
        assert!(is_file_uri(&uri));

        let uri = Uri::from_static("tmp");
        assert!(is_file_uri(&uri));
    }

    #[test]
    fn test_new_client() {
        let uri = Uri::from_static("http://example.com");
        let client = Client::new(uri);

        match client {
            Client::NotImplementedYet {..} => assert!(true),
            _ => assert!(false),
        };

        let uri = Uri::from_static("/tmp");
        let client = Client::new(uri);

        match client {
            Client::FileStorageClient { .. } => assert!(true),
            _ => assert!(false)
        };

        let uri = Uri::from_static("tmp");
        let client = Client::new(uri);
        match client {
            Client::FileStorageClient { .. } => assert!(true),
            _ => assert!(false)
        };

    }

    #[tokio::test]
    async fn test_client_set_get() {
        let current_directory = std::env::current_dir().expect("Failed to get current directory"); 
        let dir_path = format!("{}/client_set_get", current_directory.display());
        let uri = Uri::builder().path_and_query(dir_path.clone()).build().unwrap();
        let client = Client::new(uri);
        match client {
            Client::FileStorageClient { .. } => assert!(true),
            _ => assert!(false)
        };

        let key = "hi";
        let value = serde_json::json!({"k": "v"});
        let set = client.set(key, value.clone()).await;
        assert!(set.is_ok());

        let get = client.get(key).await;
        assert!(get.is_some());
        assert_eq!(get.unwrap(), value.clone());

        // clean up
        tokio::fs::remove_dir_all(dir_path.clone()).await.expect("Failed to remove directory");
    }

}
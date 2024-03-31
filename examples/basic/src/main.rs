#![allow(unused_variables)]

use tado::{Auth, Client, Configuration};

#[tokio::main]
async fn main() {
    let mut client = Client::new(Configuration {
        auth: Auth {
            username: "x".to_string(),
            password: "x".to_string(),
        },
    });

    // Try authentication and fetch current user
    let _ = client.login().await;

    let me = client.get_me().unwrap();
    let home = client.get_home().await.unwrap();
    let zones = client.get_zones().await.unwrap();

    // Show HI on device
    let _ = client
        .set_identify(&zones[0].devices[0].serial_no)
        .await
        .unwrap();

    // ... other methods, check the documentation for more
}

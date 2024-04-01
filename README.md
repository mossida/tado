# tado

[![Crates.io](https://img.shields.io/crates/v/tado.svg)](https://crates.io/crates/tado)

The `tado` crate provides bindings and methods for utilizing the (unofficial) Tado API in Rust.

*   Built on reqwest for HTTP requests
*   Utilizes tokio for asynchronous operations
*   Implements oauth2 for handling authentication
*   Supports chrono types for managing dates and timezones
*   Get/Set methods do not require mutability
*   Designed for use in concurrent environments
*   Maintains a small dependency tree

We aim to offer the most comprehensive bindings available on the web, incorporating types and requests from various open-source contributors. A special thanks to all those who have contributed to this effort.

## Examples

Basic usage

```rust
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

    // Fetch basic entities
    let me = client.get_me().await.unwrap();
    let home = client.get_home().await.unwrap();
    let zones = client.get_zones().await.unwrap();

    // Show HI on device
    let _ = client
        .set_identify(&zones[0].devices[0].serial_no)
        .await
        .unwrap();
}
```

## Collaborating

While we strive to provide the latest bindings available, our ability to do so is limited by the hardware at our disposal. As a result, some types may be incomplete or entirely missing in version 1.x.

We welcome contributions from the community. If you find areas for improvement or wish to propose changes, please feel free to open a pull request (PR). Your contributions are greatly appreciated.

## License

This project is licensed under the Apache 2.0 license.

### Special thanks

* [libtado](https://github.com/germainlefebvre4/libtado)
* [node-tado-client](https://github.com/mattdavis90/node-tado-client)
* [TadoApi](https://github.com/KoenZomers/TadoApi)
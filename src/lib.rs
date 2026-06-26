pub mod auth_provider;
pub mod capture;
pub mod config;
pub mod input_controller;
pub mod protocol;
pub mod server;

#[cfg(all(feature = "auth_provider_pam", not(target_os = "linux")))]
compile_error!("The feature auth_provider_pam only works on linux");

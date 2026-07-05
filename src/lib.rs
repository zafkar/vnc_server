pub mod auth_provider;
pub mod capture;
pub mod config;
pub mod input_controller;
#[cfg(feature = "management")]
pub mod mgmt_server;
pub mod protocol;
pub mod server;

#[cfg(all(feature = "auth_provider_pam", not(target_os = "linux")))]
compile_error!("The feature auth_provider_pam only works on linux");

#[cfg(all(feature = "auth_provider_winlogon", not(target_os = "windows")))]
compile_error!("The feature auth_provider_winlogon only works on linux")
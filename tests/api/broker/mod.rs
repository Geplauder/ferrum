//! We have a separate module for each [`ferrum_shared::broker::BrokerEvent`] enum item.
//!
//! Within these modules, we test that publishing a [`ferrum_shared::broker::BrokerEvent`]
//! results in certain [`ferrum_websocket::messages::SerializedWebSocketMessage`], depending
//! on certain situations.

pub mod delete_channel;
pub mod delete_server;
pub mod new_channel;
pub mod new_message;
pub mod new_server;
pub mod update_channel;
pub mod update_server;
pub mod user_joined;
pub mod user_left;

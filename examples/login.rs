use std::{env, process::exit};
use url::Url;

use matrix_sdk::{
    self, async_trait,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        SyncMessageEvent,
    },
    Client, ClientConfig, EventEmitter, SyncRoom, SyncSettings,
};
use dotenv::dotenv;

struct EventCallback;

#[async_trait]
impl EventEmitter for EventCallback {
    async fn on_room_message(&self, room: SyncRoom, event: &SyncMessageEvent<MessageEventContent>) {
        if let SyncRoom::Joined(room) = room {
            if let SyncMessageEvent {
                content: MessageEventContent::Text(TextMessageEventContent { body: msg_body, .. }),
                sender,
                ..
            } = event
            {
                let name = {
                    // any reads should be held for the shortest time possible to
                    // avoid dead locks
                    let room = room.read().await;
                    let member = room.joined_members.get(&sender).unwrap();
                    member.name()
                };
                println!("{}: {}", name, msg_body);
            }
        }
    }
}

async fn login(hs:String, uname:String, pw:String) -> Result<(), matrix_sdk::Error> {
    let client_config = ClientConfig::new();
    let homeserver_url = Url::parse(hs).expect("Couldn't parse the homeserver URL");
    let mut client = Client::new_with_config(homeserver_url, client_config).unwrap();
    client.add_event_emitter(Box::new(EventCallback)).await;
    client
        .login(uname, pw, None, None)
        .await?;
    client.sync(SyncSettings::new()).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), matrix_sdk::Error> {
    dotenv().ok();
    let homeserver: String = env::var("HOMESERVER").unwrap();
    let username: String = env::var("USERNAME").unwrap();
    let password: String = env::var("PASSWORD").unwrap();
    login(homeserver, username, password).await
}

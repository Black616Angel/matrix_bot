use std::{env, process::exit};

use matrix_sdk::{
    self, async_trait,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        AnyMessageEventContent, SyncMessageEvent,
        room::member::MemberEventContent, StrippedStateEvent,
    },
    Client, ClientConfig, EventEmitter, JsonStore, SyncRoom, SyncSettings,
    identifiers::user_id,

};
use dotenv::dotenv;
use tokio::time::{sleep, Duration};
use url::Url;

struct CommandBot {
    /// This clone of the `Client` will send requests to the server,
    /// while the other keeps us in sync with the server using `sync`.
    client: Client,
    username: String,
}

impl CommandBot {
    pub fn new(client: Client, username: String) -> Self {
        Self { client, username }
    }

    fn process_msg(msg_body: String) -> String {
        if !msg_body.get(..2).is_some() {
            return "".to_string()
        }
        if msg_body.get(..1).unwrap() != "!" {
            return "".to_string()
        }
        let task = msg_body.get(1..).unwrap();
        if task == "dice" {
            let x = rand::random::<u64>();
            let ant = (x % 60).to_string();
            return ant
        }
        task.to_string()
    }
}

#[async_trait]
impl EventEmitter for CommandBot {
    async fn on_room_message(&self, room: SyncRoom, event: &SyncMessageEvent<MessageEventContent>) {
        if let SyncRoom::Joined(room) = room {
            let usid = user_id!("@example:localhost");
            let ( msg_body, sender ) = if let SyncMessageEvent {
                content: MessageEventContent::Text(TextMessageEventContent { body: msg_body, .. }),
                sender,
                ..
            } = event
            {
                ( msg_body.clone(), sender )
            } else {
                ( String::new(), &usid )
            };
            let name = {
                    // any reads should be held for the shortest time possible to
                    // avoid dead locks
                    let room = room.read().await;
                    let member = room.joined_members.get(&sender).unwrap();
                    member.name()
                };
            println!("{:?} : {:?}", name, self.username);
            if name == self.username {
                return;
            }
            let response = CommandBot::process_msg(msg_body);
            if response != "" {
                let content = AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(response,
                ));
                // we clone here to hold the lock for as little time as possible.
                let room_id = room.read().await.room_id.clone();

                println!("sending");

                self.client
                    // send our message to the room we found the "!party" command in
                    // the last parameter is an optional Uuid which we don't care about.
                    .room_send(&room_id, content, None)
                    .await
                    .unwrap();

                println!("message sent");
            }
        }
    }
    async fn on_stripped_state_member(
        &self,
        room: SyncRoom,
        room_member: &StrippedStateEvent<MemberEventContent>,
        _: Option<MemberEventContent>,
    ) {
        if room_member.state_key != self.client.user_id().await.unwrap() {
            return;
        }

        if let SyncRoom::Invited(room) = room {
            let room = room.read().await;
            println!("Autojoining room {}", room.room_id);
            let mut delay = 2;

            while let Err(err) = self.client.join_room_by_id(&room.room_id).await {
                // retry autojoin due to synapse sending invites, before the
                // invited user can join for more information see
                // https://github.com/matrix-org/synapse/issues/4345
                eprintln!(
                    "Failed to join room {} ({:?}), retrying in {}s",
                    room.room_id, err, delay
                );

                sleep(Duration::from_secs(delay)).await;
                delay *= 2;

                if delay > 3600 {
                    eprintln!("Can't join room {} ({:?})", room.room_id, err);
                    break;
                }
            }
            println!("Successfully joined room {}", room.room_id);
        }
    }
}

async fn login_and_sync(
    homeserver_url: String,
    username: String,
    password: String,
) -> Result<(), matrix_sdk::Error> {
    // the location for `JsonStore` to save files to
    let mut home = dirs::home_dir().expect("no home directory found");
    home.push("party_bot");

    let store = JsonStore::open(&home)?;
    let client_config = ClientConfig::new()
        .state_store(Box::new(store));
    let homeserver = homeserver_url.to_string();
    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    // create a new Client with the given homeserver url and config
    let mut client = Client::new_with_config(homeserver_url, client_config).unwrap();

    client
        .login(&username, &password, None, Some("command bot"))
        .await?;

    println!("logged in as {}", username);

    // An initial sync to set up state and so our bot doesn't respond to old messages.
    // If the `StateStore` finds saved state in the location given the initial sync will
    // be skipped in favor of loading state from the store
    client.sync_once(SyncSettings::default()).await.unwrap();
    // add our CommandBot to be notified of incoming messages, we do this after the initial
    // sync to avoid responding to messages before the bot was running.
    let homeserver = homeserver.replace("https://matrix.", "");
    let uname = "@".to_string() + &username + &":".to_string() + &homeserver;
    client
        .add_event_emitter(Box::new(CommandBot::new(client.clone(), uname)))
        .await;

    // since we called `sync_once` before we entered our sync loop we must pass
    // that sync token to `sync`
    let settings = SyncSettings::default().token(client.sync_token().await.unwrap());
    // this keeps state from the server streaming in to CommandBot via the EventEmitter trait
    client.sync(settings).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), matrix_sdk::Error> {
    tracing_subscriber::fmt::init();

    
    dotenv().ok();
    let homeserver: String = env::var("HOMESERVER").unwrap();
    let username: String = env::var("USERNAME").unwrap();
    let password: String = env::var("PASSWORD").unwrap();
    login_and_sync(homeserver, username, password).await?;
    Ok(())
}

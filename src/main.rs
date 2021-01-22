use matrix_sdk::{
    self, async_trait,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        AnyMessageEventContent, SyncMessageEvent,
        room::member::MemberEventContent, StrippedStateEvent,
    },
    Client, EventEmitter, SyncRoom, SyncSettings,
    identifiers::user_id,

};
use tokio::time::{sleep, Duration};
use url::Url;

use std::env;
use dotenv::dotenv;

pub mod dice;
use crate::dice::DiceBot;

struct CommandBot {
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
        let mut tasks : std::str::SplitN<&str> = msg_body.get(1..).unwrap().splitn(2, " ");
        let task : Option<&str> = tasks.next();
        let rest : Option<&str> = tasks.next();
        if task == None { return "".to_string() }
        let rest = match rest {
        	Some(rest) => rest.to_string(),
        	None	   => "".to_string(),
        };
        let task : &str = &task.unwrap();
        let ant : String = match task {
         "dice" => {
         	DiceBot::dice(rest)
        },
        "roll" => {
         	DiceBot::roll(rest)
        }
        _ => "".to_string(),
    };

        ant.to_string()
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
                    let room = room.read().await;
                    let member = room.joined_members.get(&sender).unwrap();
                    member.name()
                };
            if name == self.username {
                return;
            }
            let response = CommandBot::process_msg(msg_body);
            if response != "" {
                let content = AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(response,
                ));
                // we clone here to hold the lock for as little time as possible.
                let room_id = room.read().await.room_id.clone();
                self.client
                    .room_send(&room_id, content, None)
                    .await
                    .unwrap();
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
        }
    }
}

async fn login_and_sync(
    homeserver_url: String,
    username: String,
    password: String,
) -> Result<(), matrix_sdk::Error> {
    let uname = "@".to_string() + &username + &":".to_string() + &(homeserver_url.to_string().replace("https://matrix.", ""));
    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let mut client = Client::new(homeserver_url).unwrap();

    client.login(&username, &password, None, Some("command bot")).await?;
    client.sync_once(SyncSettings::default()).await.unwrap();
    client.add_event_emitter(Box::new(CommandBot::new(client.clone(), uname))).await;
    let settings = SyncSettings::default().token(client.sync_token().await.unwrap());
    client.sync(settings).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), matrix_sdk::Error> {
    dotenv().ok();
    let homeserver: String = env::var("HOMESERVER").unwrap();
    let username: String = env::var("USERNAME").unwrap();
    let password: String = env::var("PASSWORD").unwrap();
    login_and_sync(homeserver, username, password).await?;
    Ok(())
}

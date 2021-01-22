use std::{env, process::exit};
use tokio::time::{sleep, Duration};

use matrix_sdk::{
    self, async_trait,
    events::{room::member::MemberEventContent, StrippedStateEvent},
    Client, ClientConfig, EventEmitter, SyncRoom, SyncSettings,
};
use url::Url;
use dotenv::dotenv;

struct AutoJoinBot {
    client: Client,
}

impl AutoJoinBot {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl EventEmitter for AutoJoinBot {
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
    username: &str,
    password: &str,
) -> Result<(), matrix_sdk::Error> {
    //let mut home = dirs::home_dir().expect("no home directory found");
   // home.push("autojoin_bot");

    let client_config = ClientConfig::new(); //.store_path(home);

    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let mut client = Client::new_with_config(homeserver_url, client_config).unwrap();

    client
        .login(username, password, None, Some("autojoin bot"))
        .await?;

    println!("logged in as {}", username);

    client
        .add_event_emitter(Box::new(AutoJoinBot::new(client.clone())))
        .await;

    client.sync(SyncSettings::default()).await;

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

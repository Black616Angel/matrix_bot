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
pub mod admin;
pub mod cbot;
use crate::dice::DiceBot;
use crate::admin::AdminBot;
use crate::cbot::CustomBot;

struct CommandBot {
    client: Client,
    username: String,
    admin: AdminBot,
	custombots: Vec<CustomBot>,
}

impl CommandBot {
    pub fn new(client: Client, username: String) -> Self {
    	let admin: AdminBot;
    	if env::var("ADMIN_USERS").is_ok() {
	    	admin = AdminBot::new(env::var("ADMIN_USERS").unwrap());
	    } else {
	    	admin = AdminBot::new("@exampleuser:example.example".to_string());
	    }

    	//for now bots have to rest in the bots.json
    	let custombots = CustomBot::newVec(std::fs::read_to_string("bots.json").unwrap().parse().unwrap());
        Self { client, username, admin, custombots }
    }

    fn process_msg(&self, msg_body: String, user: String) -> String {
		//check if message contains more than 1 char
        if !msg_body.get(..2).is_some() {
            return "".to_string()
        }
        //is the first char a !?
        if msg_body.get(..1).unwrap() != "!" {
            return "".to_string()
        }

        //split the message after the first char into 2 parts:
        // the first part is the kind of bot, we want
        // the second part is what that bot should do
        // each bot must have those 2 things at least (optionally then there are the other arguments, the user may add)
        let mut tasks: std::str::SplitN<&str> = msg_body.get(1..).unwrap().splitn(2, " ");
        let task: Option<&str> = tasks.next();
        let rest: Option<&str> = tasks.next();
        if task == None { return "".to_string() }
        let rest = match rest {
        	Some(rest) => rest.to_string(),
        	None	   => "".to_string(),
        };
        let task : &str = &task.unwrap();

		//it's a work in progress, but it will result in only the custombots, I think
        let ant : String = match task {
         "dice" => {
         	DiceBot::dice(rest)
        },
        "roll" => {
         	DiceBot::roll(rest)
        },
        "admin" => {
        	self.admin.admin(rest, user)
        }
        _ => {
        	self.checkCBs(task.to_string(), rest, user)
        },
    };

        ant.to_string()
    }
    fn checkCBs(&self, task: String, rest: String, user: String) -> String {
    	//we lop over all the bots
    	let mut cbIter = self.custombots.iter();
    	let mut oCbot = cbIter.next();
    	while oCbot.is_some() {
    		if oCbot.unwrap().name == task {
    			return oCbot.unwrap().callCommand(task, rest, user); //hopefully find the right one
    		};
    		oCbot = cbIter.next();
    	};
    	
    	return "no bot matched the task".to_string()
    }
}

#[async_trait]
impl EventEmitter for CommandBot {

	//event for messages
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
            let response = self.process_msg(msg_body, name);
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

    //this is the event for the room invite
    // by now we just join all the rooms
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


//simple login with the one bot-user
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
    //get infos from the environment
    //this might change later, but I am too lazy right now :D
    let homeserver: String = env::var("HOMESERVER").unwrap();
    let username: String = env::var("USERNAME").unwrap();
    let password: String = env::var("PASSWORD").unwrap();
    login_and_sync(homeserver, username, password).await?;
    Ok(())
}

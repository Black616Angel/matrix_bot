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
use futures::future;
use url::Url;

pub mod cbot;
use crate::cbot::CustomBot;

struct CommandBot{
	pub client: Client,
	pub custombot: CustomBot,
}

impl CommandBot {
	pub async fn new_vec(bots_json: String) -> Vec<Client> {
    	let mut bots = CustomBot::new_vec(bots_json);
		let mut ret: Vec<Client> = Vec::new();
		//we loop over all the bots in the JSON
		let mut o_custombot = bots.pop();
		while o_custombot.is_some() {
			let custombot = o_custombot.unwrap();
			//build the username: @username:matrix.example.com
		    let uname = "@".to_string() + &custombot.username + &":".to_string() + &(custombot.homeserver.to_string().replace("https://matrix.", ""));
		    let homeserver_url = Url::parse(&custombot.homeserver).expect("Couldn't parse the homeserver URL");
		    
		    //now create the client
		    let mut client = Client::new(homeserver_url).unwrap();
		    //enter username, password, idc, devicename
		    client.login(&uname, &custombot.password, None, Some("command bot")).await.unwrap();
		    client.sync_once(SyncSettings::default()).await.unwrap();
		    //make new bot the event handler
		    client.add_event_emitter(Box::new(Self{ client: client.clone(), custombot })).await;
			ret.push( client );
			o_custombot = bots.pop();
		}
		return ret
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

		self.check_cb(task.to_string(), rest, user)
    }
    fn check_cb(&self, task: String, rest: String, user: String) -> String {
   		if self.custombot.name == task {
			let mut command = rest.splitn(2, " ");
	        let task: String = match command.next() {
	        	Some(s) => s.to_string(),
	        	None	=> "".to_string(),
	        };
	        let rest: String = match command.next() {
	        	Some(s) => s.to_string(),
	        	None	=> "".to_string(),
	        };
  			return self.custombot.call_command(task, rest, user);
   		};
    	return "".to_string()
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
                    let member = room.joined_members.get(&sender);
                    if member.is_some() {
                    	member.unwrap().name()
                    } else {
                    	"".to_string()
                    }
                };
            if name == self.custombot.username {
                return;
            }
            let response = self.process_msg(msg_body, name);
            if response != "" {
                let content = AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(response, ));
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
            //println!("Autojoining room {}", room.room_id);
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

#[tokio::main]
async fn main() -> Result<(), matrix_sdk::Error> {
    //for now bots have to rest in the bots.json
	let bots_json = std::fs::read_to_string("bots.json").unwrap().parse().unwrap();
	let clients = CommandBot::new_vec(bots_json).await;
	let mut f_clients: Vec<future::BoxFuture<()>> = Vec::new();
	for client in clients.iter() {
		let settings = SyncSettings::default().token(client.sync_token().await.unwrap());
		f_clients.push(Box::pin(client.sync(settings)));
	}
	future::join_all(f_clients).await;
	
    Ok(())
}

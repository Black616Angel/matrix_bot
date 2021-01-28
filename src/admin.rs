use std::process::Command;

pub struct AdminBot {
	pub users: Vec<String>,
}

impl AdminBot {
	pub fn new(users: String) -> AdminBot {
		let str_users: Vec<&str> = users.split(";").collect();
		let users: Vec<String> = str_users.iter().map(|val| val.to_string()).collect();
		AdminBot{ users }
	}

	pub fn admin(&self, message: String, user: String) -> String {
		if !self.users.contains(&user) {
			return "Not allowed 😠".to_string()
		}

		//get keywords and other stuff from user message
		let keyword: &str;
		let rest: &str;
		if message.contains(" ") {
			let mut strsplit = message.splitn(2, " ");
			keyword = strsplit.next().unwrap();
			rest    = strsplit.next().unwrap();
		} else {
			keyword = &message;
			rest = "";
		}
		
		let ret: String;
		ret = match keyword {
			"reboot" => self.reboot(),
			"ts3" => {
				match rest {
					"restart" => self.ts3("restart").to_string(),
					"status"  => self.ts3("status").to_string(),
					"start"   => self.ts3("start").to_string(),
					_         =>"unknown ts3-command".to_string(),
				}
			},
			_ => "command unknown".to_string(),
		};
		ret
	}
	fn reboot(&self) -> String {

		//build the command and ignore the output, we don't need that...
		let err = Command::new("sudo").args(&["shutdown", "-r", "-t", "sec", "5"]).spawn();
		if err.is_err() {
			return "rebooting didn't work...".to_string()
		} else {
			return "rebooting in 5 Seconds".to_string()
		}
	}
	fn ts3(&self, arg: &str) -> String {
		//here we want the output, so we get it
		let err = Command::new("sudo").args(&["-u", "ts3", "/home/ts3/teamspeak3-server_linux_amd64/ts3server_startscript.sh", arg]).output();
		if err.is_err() {
			return "now that didn't work...".to_string()
		} else {
			let err2 = err.unwrap();
			let ret = std::str::from_utf8(&err2.stdout);
			if ret.is_err() {
				return "something went wrong here.".to_string()
			} else {
				return ret.unwrap().to_string()
			}
		}
	}
}

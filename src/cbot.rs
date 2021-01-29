use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize)]
pub struct CustomBot {
	pub name: String,
	pub commands: Vec<Cmd>,
}

#[derive(Serialize, Deserialize)]
pub struct Cmd{
	pub name: String,
	pub sudo: bool,
	pub exec: String,
	pub need_user: bool,
	pub param_count: usize,
}

impl CustomBot {
	pub fn newVec(sJson: String) -> Vec<CustomBot> {
		serde_json::from_str(&sJson).unwrap()
		
	}

	pub fn new(sJson: String) -> CustomBot {
		serde_json::from_str(&sJson).unwrap()
	}

	pub fn callCommand(&self, name: String, args: String, user: String) -> String {

		//get command info
		//println!("{:?}", name);
		let cmdo: Option<&Cmd> = self.commands.iter().find(|c| c.name == name);
		let cmd: &Cmd;
		match cmdo {
			Some(c) => { cmd = c; },
			None 	=> return "command unknown".to_string(),
		}

		//now build the command and ...
		let mut comm = if cmd.sudo {
							Command::new("sudo")
						} else {
							Command::new(&cmd.exec)
						};
		if cmd.sudo {
			comm.arg(&cmd.exec);
		}
		if cmd.need_user {
			comm.arg(user);
		}
		if cmd.param_count > 0 {
			//arguments are space-separated
			let mut args: std::str::SplitN<&str> = args.splitn(cmd.param_count, " ");
			//we loop over all the arguments
			for _n in 1..cmd.param_count {
				match args.next() { //just add the arguments
					Some(arg) => { comm.arg(arg); },
					// if the numbers don't match we end the process
					None	  => return "insuffient arguments supplied".to_string(),
				}
			}
			
			
		}

		//send it to the shell and catch the output
		//println!("{:?}", comm);
		let err = comm.output();
		
		//error-handling and output
		if err.is_err() {
			return "now that didn't work...".to_string()
		} else {
			let err2 = err.unwrap();
			let ret = std::str::from_utf8(&err2.stdout);//this converts the shell output, which is in binary into a String
			if ret.is_err() {//if it works of course :D
				return "something went wrong here.".to_string()
			} else {
				return ret.unwrap().to_string()
			}
		}
	}

}

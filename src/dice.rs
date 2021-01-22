use std::convert::TryInto;
pub struct DiceBot {

}

impl DiceBot {
	pub fn dice(dice: String) -> String {
        let x = rand::random::<u64>();
        let d : u64 =  {
        		let ret = dice.parse::<u64>();
        		let re : u64 = if ret.is_err() { 6 }
        		else { ret.unwrap() };
        		re
        };
        (1 + x % d).to_string()
	}

	pub fn roll(dice: String) -> String {
		let dice = dice.replace(" ", "");
		return DiceBot::calc_dice(dice).to_string();
	}

	pub fn calc_dice(term: String) -> i64 {
		println!("{:?}", term);
		if term.contains('+') {
			let mut summands = term.split("+");
			let mut summand = summands.next();
			let mut sum : i64 = 0;
			while summand != None {
				sum = sum + DiceBot::calc_dice(summand.unwrap().to_string());
				summand = summands.next();
			}
			println!("sum: {:?}", sum);
			return sum
		} else if term.contains('-') {
			let mut minuends = term.split("-");
			let mut subtrahend = minuends.next();
			let mut difference : i64 = DiceBot::calc_dice(subtrahend.unwrap().to_string());
			subtrahend = minuends.next();
			while subtrahend != None {
				difference = difference - DiceBot::calc_dice(subtrahend.unwrap().to_string());
				subtrahend = minuends.next();
			}
			println!("difference: {:?}", difference);
			return difference
		} else if term.contains("d") {
			let mut dice = term.splitn(2, "d");
			let num : i64 = match dice.next() {
				Some(num) => {
					if num.parse::<i64>().is_err() { 1 }
					else { num.parse().unwrap() }
				},
				None => 1 
			};
			let die : u32 = match dice.next() {
				Some(num) => {
					if num.parse::<u32>().is_err() { 1 }
					else { num.parse().unwrap() }
				},
				None => 1
			};
        	let x = rand::random::<u32>();
        	let die: i64 = (1 + x % die).try_into().unwrap();
			println!("die throw: {:?}", num * die);
			num * die
		} else {
			let ret = term.parse();
			let ret = if ret.is_err() {
				0
			} else {
				ret.unwrap()
			};
			println!("ret: {:?}", ret);
			return ret
		}
	}
}
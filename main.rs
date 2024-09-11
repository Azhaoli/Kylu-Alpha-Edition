/*
                _---___
           __  /       \
          /  \ \       / __
         /___| |______/ /  \
          ___| |______ |   |
         \   | |      \ \__/
          \__/ /       \
               \_    __/
                  ~~~

           KYLU PROJECT a2.3
             Now in Rust!
*/

mod parser;
mod evaluator;
mod utils;

use std::{ env, process, io };
use std::io::Write;
use crate::evaluator::{ Env, eval_file, load_file };
use crate::utils::node::Node;


fn run (file_name: String) {
	let mut source_file: String = load_file(file_name, true);
	let env = Env::create();
	eval_file(env, source_file, true);
}


fn get_input(prompt: &str) -> String {
	let mut command = String::new();
	print!("{}", prompt);
	io::stdout().flush().unwrap();
	io::stdin().read_line(&mut command).unwrap();
	command
}


fn terminal() {
    println!(
r"
---------------------------------------------------------------------------------

		                                          _---___
		                                     __  /       \
		                                    /  \ \       / __
		KYLU PROJECT TERMINAL              /___| |______/ /  \
		pre-release build alpha 2.3         ___| |______ |   |
		                                   \   | |      \ \__/
		Azhaoli c2024                       \__/ /       \
		                                         \_    __/
		                                            ~~~

---------------------------------------------------------------------------------
"
	);
	let env = Env::create();
	loop {
		let mut command = get_input("(kylu2.3)--> ");
		if command.chars().nth(0).unwrap() == '/' {
			let mut comm_iter = command.trim().split(" ");
			match comm_iter.next() {
				Some("/exit") | Some("/x") => {
					println!("[-] program stopped");
					process::exit(0);
				},
				Some("/edit") => {
					command = String::new();
					loop {
						let append = get_input("> ");
						if append == "/done\n" { break; }
						command.push_str(&append);
				}},
				Some("/bindings") | Some("/bind") => {
					println!("--------------------------------------------------------- LOCAL BINDINGS --------");
					println!("{}", env.data[0].show());
					continue;
				},
				Some("/extensions") | Some("/ext") => {
					println!("------------------------------------------------------ IMPORTED BINDINGS --------");
					env.import.show_modules();
					continue;
				},
				Some("/load") => {
					let path = match comm_iter.next() {
						None => { 
							println!("[-] specify file path to load");
							continue;
						},
						Some(path) => path
					};
					let source = load_file(path.to_string(), false);
					if source == String::new() { continue; }
					
					let guest_env = Env::create();
					eval_file(guest_env.clone(), source, false); // evaluate file contents
					
					let file_name = path.split("/").collect::<Vec<&str>>().pop().unwrap(); // get last arg in path
					let mut label = file_name.split(".").nth(0).unwrap(); // remove extension
					env.import.set(Node::symbol(label.to_string()), guest_env.data[0].as_node(Node::symbol("<extension>".to_string())));

					println!("[+] loaded file: {}", file_name);					
					continue;
				}
				Some(inv) => {
					println!("[-] invalid terminal command: {}", inv);
					continue;
				},
				None => { continue; }
		}}
		eval_file(env.clone(), command, false);
		println!("");
	}
}


fn main () {
    match env::args().nth(1) {
        Some(file) => run(file),
        None => terminal()
	};
}



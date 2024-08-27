#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use vstd::{
    fs::{get_cwd, open, read},
    print, println,
};

mod commands;
mod run;
use commands::*;

fn shell_read_line(fd: usize, buf: &mut String) {
    buf.clear();

    let mut tmp_buf = [0; 1];

    read(fd, &mut tmp_buf);

    while tmp_buf[0] != b'\n' {
        if tmp_buf[0] == 8 {
            if let Some(_) = buf.pop() {
                print!("{} {}", 8 as char, 8 as char);
            }
        } else if tmp_buf[0] != 0 {
            print!("{}", tmp_buf[0] as char);
            buf.push(tmp_buf[0] as char);
        }

        read(fd, &mut tmp_buf);
    }
}

fn get_prompt() -> String {
    format!(
        "\x1b[36m[\x1b[34mroot@raca \x1b[33m{}\x1b[36m]\x1b[34m:) \x1b[0m",
        get_cwd()
    )
}

type CommandFunction = fn(args: Vec<String>);

fn exit(_args: Vec<String>) {
    vstd::task::exit(0);
}

#[no_mangle]
pub fn main() -> usize {
    let mut command_function_list = BTreeMap::<&str, CommandFunction>::new();

    {
        command_function_list.insert("cd", cd);
        command_function_list.insert("ls", ls);
        command_function_list.insert("cat", cat);
        command_function_list.insert("testfb", testfb);
        command_function_list.insert("exit", exit);
    }

    let mut input_buf = String::new();

    let fd = open(String::from("/dev/terminal"), vstd::fs::OpenMode::Read);

    loop {
        print!("{}", get_prompt());

        shell_read_line(fd, &mut input_buf);

        println!();

        let input =
            String::from_utf8(escape_bytes::unescape(input_buf.as_bytes()).unwrap()).unwrap();

        let args = input.split(" ").map(|x| x.to_string()).collect::<Vec<_>>();

        let function = command_function_list.get(&args[0].as_str());

        if let Some(function) = function {
            function(args);
        } else if let None = run::try_run(args[0].clone()) {
            println!("Command not found: {}", args[0]);
        }
    }
}

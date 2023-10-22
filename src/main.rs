use chrono::Local;
use clap::{App, Arg};
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{thread, time};

fn main() -> io::Result<()> {
    // spawn a thread to listen for messages
    let matches = App::new("Lan Chat App")
        .version("1.0")
        .author("You Name <your_email@example.com>")
        .about("Chat app for LAN")
        .arg(
            Arg::with_name("bind")
                .short("b")
                .long("bind")
                .value_name("ADDRESS")
                .help("Sets an address to bind to")
                .takes_value(true),
        )
        // Add other arguments here in the future
        .get_matches();

    let bind_addr = matches
        .value_of("bind")
        .unwrap_or("0.0.0.0:8888")
        .to_string();

    let intro_done = Arc::new(Mutex::new(false));
    let intro_done_clone = Arc::clone(&intro_done);

    let intro_thread = thread::spawn(move || {
        let intro_lines = vec![
            "Wake up, Neo...",
            "The Matrix has you...",
            "Follow the white rabbit.",
            "Knock, knock, Neo.",
        ];

        for line in intro_lines {
            print_intro_line(line);
            thread::sleep(time::Duration::from_secs(1));
        }

        *intro_done_clone.lock().unwrap() = true;
    });

    let bind_addr_clone = bind_addr.clone();
    let send_socket = UdpSocket::bind(&bind_addr_clone)?;
    let socket_clone = send_socket.try_clone()?;

    let chat_history = Arc::new(Mutex::new(Vec::new()));
    let chat_history_recv_clone = Arc::clone(&chat_history);
    let chat_history_send_clone = Arc::clone(&chat_history);

    // Receiver thread channel
    let (rx_tx, rx_rx) = mpsc::channel();

    // Sender thread channel
    let (tx_tx, tx_rx) = mpsc::channel();

    // Receiver thread
    thread::spawn(move || {
        let result = listen_for_message(&socket_clone, rx_tx);
        if let Err(e) = result {
            eprintln!("Error in listener thread: {}", e);
        }
    });

    // Sender thread
    thread::spawn(move || {
        loop {
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let input = input.trim();

            if input.is_empty() {
                print_prompt();
                continue;
            }

            if input == "exit" {
                std::process::exit(0);
            }

            let user_message = format!("you: {}", input);

            send_message(&send_socket, input).expect("Failed to send message");

            // send the user's messages to the main thread to update chat history
            tx_tx
                .send(user_message)
                .expect("Failed to send user message to main thread");
        }
    });

    // Optionally, wait for the intro to finish before proceeding
    intro_thread.join().expect("The intro thread panicked");

    clear_screen();
    print_prompt();

    // in the main thread, continuously read input and send messages
    loop {
        // check for messages from the receiver thread
        if let Ok(message) = rx_rx.try_recv() {
            chat_history_recv_clone.lock().unwrap().push(message);
            clear_screen();
            print_chat(&chat_history_recv_clone);
            print_prompt();
        }

        if let Ok(message) = tx_rx.try_recv() {
            // chat_history_send_clone.lock().unwrap().push(message);
            // clear_screen();
            // print_chat(&chat_history_send_clone);
            // print_prompt();
        }

        // sleep for a short duration to avoid busy-waiting
        thread::sleep(std::time::Duration::from_millis(100));
    }


}

fn print_intro_line(line: &str) {
    let chars: Vec<char> = line.chars().collect();
    for ch in chars {
        print!("{}", ch);
        std::io::stdout().flush().unwrap();
        thread::sleep(time::Duration::from_millis(50));
    }
    println!();  // Move to the next line after printing the current line
}


fn send_message(socket: &UdpSocket, send_msg: &str) -> io::Result<()> {
    let broadcast_address: SocketAddr = "255.255.255.255:8888".parse().unwrap();

    socket.set_broadcast(true)?;
    socket.send_to(send_msg.as_bytes(), broadcast_address)?;

    Ok(())
}

fn listen_for_message(socket: &UdpSocket, tx: mpsc::Sender<String>) -> io::Result<()> {
    let mut buffer = [0u8; 1024];
    loop {
        let (amt, src) = socket.recv_from(&mut buffer)?;
        let message = format!("{}: {}", src, String::from_utf8_lossy(&buffer[..amt]));
        log_messages(&message)?;
        tx.send(message)
            .expect("Failed to send message to main thread");
    }
}

fn log_messages(message: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("chat_log.txt")?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writeln!(file, "[{}] {}", timestamp, message)
}

fn print_chat(chat_history: &Arc<Mutex<Vec<String>>>) {
    let locked_history = chat_history.lock().unwrap();

    for message in locked_history.iter() {
        println!("{}", message);
    }
}

fn print_prompt() {
    print!("Type your message (or 'exit' to quit): ");
    io::stdout().flush().unwrap();
}

fn clear_screen() {
    #[cfg(unix)]
    {
        std::process::Command::new("clear").status().unwrap();
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cls").status().unwrap();
    }
}

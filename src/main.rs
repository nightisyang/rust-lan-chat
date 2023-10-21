use clap::{App, Arg};
use std::io;
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;


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

    let bind_addr = matches.value_of("bind").unwrap_or("0.0.0.0:0").to_string();
    let bind_addr_clone = bind_addr.clone();
    let send_socket = UdpSocket::bind(&bind_addr_clone)?;

    let mut chat_history: Vec<String> = Vec::new();

    // Receiver thread channel
    let (rx_tx, rx_rx) = mpsc::channel();

    // Sender thread channel
    let (tx_tx, tx_rx) = mpsc::channel();

    
    // Receiver thread
    thread::spawn(move || {
        let result = listen_for_message(&bind_addr, rx_tx);
        if let Err(e) = result {
            eprintln!("Error in listener thread: {}", e);
        }
    });

    // Sender thread
    thread::spawn(move || {
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read line");
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
            tx_tx.send(user_message).expect("Failed to send user message to main thread");

        }
    });

    clear_screen();
    print_prompt();

    // in the main thread, continuously read input and send messages
    loop {
        // check for messages from the receiver thread
        if let Ok(message) = rx_rx.try_recv() {
            chat_history.push(message);
            clear_screen();
            print_chat(&chat_history);
            print_prompt();
        }

        if let Ok(message) = tx_rx.try_recv() {
            chat_history.push(message);
            clear_screen();
            print_chat(&chat_history);
            print_prompt();
        }

        // sleep for a short duration to avoid busy-waiting
        thread::sleep(std::time::Duration::from_millis(100));
    }

}

fn send_message(socket: &UdpSocket, send_msg: &str) -> io::Result<()> {
    let broadcast_address: SocketAddr = "255.255.255.255:8888".parse().unwrap();

    socket.set_broadcast(true)?;
    socket.send_to(send_msg.as_bytes(), broadcast_address)?;

    Ok(())
}

fn listen_for_message(addr: &str, tx: mpsc::Sender<String>) -> io::Result<()> {
    let socket = UdpSocket::bind(addr)?; // listening on port 8888

    let mut buffer = [0u8; 1024];
    loop {
        let (amt, src) = socket.recv_from(&mut buffer)?;
        let message = format!("{}: {}", src, String::from_utf8_lossy(&buffer[..amt]));
        tx.send(message).expect("Failed to send message to main thread");
    }
}


fn print_chat(chat_history: &[String]) {
    for message in chat_history.iter() {
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


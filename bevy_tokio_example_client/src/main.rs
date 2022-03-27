use bevy::prelude::*;
use bevy_console::{ConsolePlugin, PrintConsoleLine, ConsoleCommandEntered, ConsoleOpen};
use bevy_tokio_example_core::protocol::NetworkMessage;
use tokio::sync::mpsc;
use tokio::runtime::Runtime;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::UdpSocket;
use std::sync::Arc;
use rand::prelude::SliceRandom;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ConsolePlugin)
        .insert_resource(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
        )
        .insert_resource(ConsoleOpen {
            open: true,
        })
        .add_startup_system(start_networking)
        .add_system(handle_network_messages)
        .add_system(handle_chat_message_entered)
        .run();
}

struct ChatName {
    name: String,
}

fn start_networking(
    mut commands: Commands,
    runtime: Res<Runtime>,
) {
    let (from_server_sender, from_server_receiver) = mpsc::channel::<NetworkMessage>(20);
    let (to_server_sender, mut to_server_receiver) = mpsc::channel::<NetworkMessage>(20);

    runtime.spawn(async move {
        let server_address = "127.0.0.1:9999";
        eprintln!("Connecting to server: {}", server_address);
        let remote_addresses: Vec<SocketAddr> = server_address
            .to_socket_addrs()
            .expect("Unable to resolve domain")
            .collect();
        let remote_address = remote_addresses.first().unwrap();
        let local_address = "0.0.0.0:0".parse::<SocketAddr>().unwrap();

        let socket = UdpSocket::bind(local_address).await.unwrap();
        socket.connect(&remote_address).await.unwrap();

        let read_socket = Arc::new(socket);
        let send_socket = read_socket.clone();

        let network_sends = async move {
            loop {
                if let Some(message) = to_server_receiver.recv().await {
                    eprintln!("Trying read from send_to_server queue. Got something: {:?}", message);
                    let message_json = serde_json::to_vec(&message).unwrap();
                    match send_socket.send(message_json.as_slice()).await {
                        Ok(l) => {
                            eprintln!("We sent {} bytes: {}", l, String::from_utf8_lossy(message_json.as_slice()));
                        }
                        Err(e) => {
                            eprintln!("Error sending {}", e);
                        }
                    }
                }
            }
        };

        let network_reads = async move {
            loop {
                // Read stuff from the network
                let mut data = vec![0u8; 1000];
                let maybe_len = read_socket.recv(&mut data).await;
                match maybe_len {
                    Ok(len) => {
                        let data = &data[..len];
                        let message: NetworkMessage = serde_json::from_slice(data).unwrap();
                        eprintln!("We received a message: {}", String::from_utf8_lossy(data));
                        from_server_sender.send(message).await.unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error receiving {}", e);
                    }
                }
            }
        };

        tokio::join!(network_sends, network_reads)
    });

    let us = random_name();
    to_server_sender
        .try_send(NetworkMessage::Join { name: us.name.clone() })
        .unwrap();

    commands.insert_resource(us);
    commands.insert_resource(to_server_sender);
    commands.insert_resource(from_server_receiver);
}

fn handle_network_messages(
    mut from_server: ResMut<mpsc::Receiver<NetworkMessage>>,
    mut console_line: EventWriter<PrintConsoleLine>,
) {
    while let Ok(message) = from_server.try_recv() {
        match message {
            NetworkMessage::Join { name } => {
                console_line.send(PrintConsoleLine::new(format!("{} joined", name)));
            }
            NetworkMessage::ChatMessage { from, message  } => {
                console_line.send(PrintConsoleLine::new(format!("{}: {}", from, message)));
            }
        }
    }
}

fn handle_chat_message_entered(
    name: Res<ChatName>,
    send_to_server: Res<mpsc::Sender<NetworkMessage>>,
    mut console_commands: EventReader<ConsoleCommandEntered>,
) {
    for ConsoleCommandEntered { command, args: _ } in console_commands.iter() {
        send_to_server
            .try_send(NetworkMessage::ChatMessage { from: name.name.clone(), message: command.to_string() })
            .unwrap();
    }
}

fn random_name() -> ChatName {
    let names = vec![
        "Oliver",
        "George",
        "Arthur",
        "Noah",
        "Muhammad",
        "Leo",
        "Oscar",
        "Harry",
        "Archie",
        "Henry",
        "Olivia",
        "Amelia",
        "Isla",
        "Ava",
        "Mia",
        "Ivy",
        "Lily",
        "Isabella",
        "Sophia",
        "Rosie",
    ];

    ChatName {
        name: names.choose(&mut rand::thread_rng()).unwrap().to_string(),
    }
}

use tokio::net::UdpSocket;
use bevy_tokio_example_core::protocol::NetworkMessage;
use std::net::SocketAddr;
use std::collections::VecDeque;

struct Client {
    name: String,
    address: SocketAddr,
}

#[tokio::main]
async fn main() {
    let mut socket = UdpSocket::bind("0.0.0.0:9999").await.unwrap();
    eprintln!("Game Server running on UDP port 9999");

    let mut clients = Vec::new();
    let mut messages = VecDeque::new();

    loop {
        let mut data = [0; 1024];
        let (len, address) = socket.recv_from(&mut data).await.unwrap();
        let data = &data[..len];

        eprintln!("Parsing into NetworkMessage {} from {}", String::from_utf8_lossy(&data), address);

        let network_message: NetworkMessage = serde_json::from_slice(data).unwrap();

        match network_message {
            NetworkMessage::Join { ref name } => {
                clients.push(Client {
                    name: name.to_string(),
                    address: address.clone(),
                });

                // Client just joined; send them the chat history
                for message in messages.iter().rev() {
                    // UDP does not guarantee order, so these may reach the client out of order
                    socket_send(&mut socket, message, &address);
                }

                // Client just joined; tell everyone they joined
                for client in clients.iter() {
                    socket_send(&mut socket, &network_message, &client.address);
                }
            }
            NetworkMessage::ChatMessage { from: _, message: _, } => {
                messages.push_front(network_message.clone());
                messages.truncate(10);

                // Chat message received; send it to everyone
                for client in clients.iter() {
                    socket_send(&mut socket, &network_message, &client.address);
                }
            }
        }
    }
}

pub fn socket_send(
    socket: &mut UdpSocket,
    network_message: &NetworkMessage,
    address: &SocketAddr,
) {
    match serde_json::to_vec(&network_message) {
        Ok(message_bytes) => {
            // Try to send data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_send_to(message_bytes.as_slice(), address.clone()) {
                Ok(n) => {
                    eprintln!("Sent the {} byte message {}", n, String::from_utf8_lossy(message_bytes.as_slice()));
                }
                Err(e) => {
                    eprintln!("Error sending message {:?}", e.kind());
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to serialise ChatMessage {}", e);
        }
    }
}

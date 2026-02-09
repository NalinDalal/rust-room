use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::UnboundedSender;

/// Identifier for a connected client.
pub type ClientId = usize;

/// Identifier for a room.
pub type RoomId = String;

/// A connected client.
///
/// Holds the client's id and a sending handle used to push messages
/// to the client's WebSocket task.
#[derive(Clone)]
pub struct Client {
    pub id: ClientId,
    pub sender: UnboundedSender<String>,
}

/// A simple room with a set of connected client ids.
pub struct Room {
    pub _id: RoomId, // unused, but kept for future use
    pub clients: HashSet<ClientId>,
}

/// Manages rooms and clients.
///
/// Example:
///
/// ```no_run
/// use rust_room::room::{RoomManager, Client};
/// let mut mgr = RoomManager::new();
/// // join and broadcast demonstrated in integration tests or runtime
/// ```
pub struct RoomManager {
    pub rooms: HashMap<RoomId, Room>,
    pub clients: HashMap<ClientId, Client>,
}

impl RoomManager {
    /// Create an empty `RoomManager`.
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    /// Add a `Client` to a room (creating it if missing).
    pub fn join_room(&mut self, room_id: &str, client: Client) {
        let room = self.rooms.entry(room_id.to_string()).or_insert(Room {
            _id: room_id.to_string(),
            clients: HashSet::new(),
        });
        room.clients.insert(client.id);
        self.clients.insert(client.id, client);
    }

    /// Remove a client from a room and drop the client mapping.
    pub fn leave_room(&mut self, room_id: &str, client_id: ClientId) {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.clients.remove(&client_id);
        }
        self.clients.remove(&client_id);
    }

    /// Broadcast a `message` to all clients in `room_id`.
    pub fn broadcast(&self, room_id: &str, message: &str) {
        if let Some(room) = self.rooms.get(room_id) {
            for client_id in &room.clients {
                if let Some(client) = self.clients.get(client_id) {
                    let _ = client.sender.send(message.to_string());
                }
            }
        }
    }
}

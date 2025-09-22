use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::UnboundedSender;

pub type ClientId = usize;
pub type RoomId = String;

#[derive(Clone)]
pub struct Client {
    pub id: ClientId,
    pub sender: UnboundedSender<String>,
}

pub struct Room {
    pub _id: RoomId, // unused, but kept for future use
    pub clients: HashSet<ClientId>,
}

pub struct RoomManager {
    pub rooms: HashMap<RoomId, Room>,
    pub clients: HashMap<ClientId, Client>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    pub fn join_room(&mut self, room_id: &str, client: Client) {
        let room = self.rooms.entry(room_id.to_string()).or_insert(Room {
            _id: room_id.to_string(),
            clients: HashSet::new(),
        });
        room.clients.insert(client.id);
        self.clients.insert(client.id, client);
    }

    pub fn leave_room(&mut self, room_id: &str, client_id: ClientId) {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.clients.remove(&client_id);
        }
        self.clients.remove(&client_id);
    }

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

use uuid::Uuid;

struct Profile {
    uuid: Uuid,
    user_id: Uuid,
    room_id: Uuid,

    handle: String,
    alias: String,

    // unique: uuid
    // unique: user_id, room_id
    // unique: handle, room_id
}

enum RoomVisibility {
    Private,
    Public,
}

struct Room {
    uuid: Uuid,

    name: String,
    visibility: RoomVisibility,

    // unique: uuid
}

struct Message {
    id: Uuid,
    room_id: Uuid,
    
    profile_id: Uuid,
    reply_to_id: Uuid,

    content: String,

    // unique: id, room_id
}
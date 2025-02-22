CREATE TABLE profiles (
    uuid UUID NOT NULL UNIQUE,
    open_id UUID NOT NULL,
    room_id UUID NOT NULL,
    alias TEXT NOT NULL,
    UNIQUE(open_id, room_id)
);

CREATE TABLE rooms (
    uuid UUID NOT NULL UNIQUE,
    name TEXT NOT NULL,
    msgs_json UUID NOT NULL
);

INSERT INTO rooms (uuid,name,msgs_json) VALUES ("67e55044-10b1-426f-9247-bb680e5fe0c8","OG Server","[{pid:69,msg:'hello'}]")
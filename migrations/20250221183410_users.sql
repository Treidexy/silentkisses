create table profiles (
    uuid text not null unique,

    user_id text not null,
    room_id text not null,

    handle text not null,
    alias text not null,

    unique(user_id, room_id),
    unique(handle, room_id)
);

create table rooms (
    uuid text not null unique,

    name text not null,

    msgs_json json not null
);

insert into rooms (uuid,name,msgs_json) values ("67e55044-10b1-426f-9247-bb680e5fe0c8","OG Room","[{pid:69,msg:'hello'}]")
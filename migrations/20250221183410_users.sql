create table profiles (
    -- pid
    uuid text not null unique,

    -- uid
    user_id text not null,
    -- rid
    room_id text not null, 

    handle text not null,
    alias text not null,

    unique(user_id, room_id),
    unique(handle, room_id)
) strict;

create table rooms (
    -- rid
    uuid text not null unique,

    name text not null,
    is_public integer not null
) strict;

create table messages (
    -- mid
    id text not null,
    -- rid
    room_id text not null,

    -- pid
    profile_id text not null,
    -- rtid
    reply_to_id text,
    
    content text not null,

    unique(id, room_id)
) strict;

insert into rooms (uuid, name, is_public) values (
    "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "OG Room",
    1
);
insert into profiles (uuid, user_id, room_id, handle, alias) values (
    "f3f2e850-b5d4-11ef-ac7e-96584d5248b2",
    "smileyface",
    "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "smileyface",
    "A Happy Fella"
);
insert into messages (id, room_id, profile_id, reply_to_id, content) values (
    "9c5b94b1-35ad-49bb-b118-8e8fc24abf80",
    "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "f3f2e850-b5d4-11ef-ac7e-96584d5248b2",
    null,
    "**Hello** world to a *happy* day!"
), (
    "8D8AC610-566D-4EF0-9C22-186B2A5ED793",
    "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "f3f2e850-b5d4-11ef-ac7e-96584d5248b2",
    "9c5b94b1-35ad-49bb-b118-8e8fc24abf80",
    "I cannot ***wait*** to see you!"
);
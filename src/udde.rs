use num_derive::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum client_msgs {
    Greeting, // GREETING => \
    Log,      // LOG => log::Level as u8 => msg length as usize => log message
    BatchRequest,    // BATCH_REQUEST => \
    JSON,     // JSON => json length as usize => serialized json from ytdlp
}


#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum server_msgs {
    Greeting,   // GREETING => \
    Batch,      // BATCH => link count as u16 => links as a string, \n is the separator
    EndRequest, // REQUEST_END => \
}

use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;

use bytes::Buf;
use bytes::BytesMut;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::{net::{UnixListener, UnixStream}, io::BufWriter};


#[tokio::main]
async fn main_tokio(){
    

    let listener = UnixListener::bind("/tmp/uds.sock").unwrap();
    let server = Server{
        listener,
    };
    let mut channelMap:HashMap<String, Connection> = HashMap::new();
    println!("accepting...");
    loop {
        let socket = server.listener.accept().await.unwrap();
        let mut connection = Connection::new(socket.0);
        connection.stream.read_buf(&mut connection.buffer).await;
        let msg = connection.read_message().await;
        
        
        match msg.unwrap() {
            Message::StringMsg(s)=>{
                channelMap.insert(s, connection);
                println!("insert one");
                for k in channelMap.iter_mut() {
                    k.1.stream.write(b"abc").await;
                    k.1.stream.flush();
                }
            },
            Message::RouteMsg(r) =>{
                let name = String::from_utf8(r.to_vec()).unwrap();
                let mut channel_conn = channelMap.get_mut(&name).unwrap();
                channel_conn.stream.write_all(b"");

                String::from("value");
            }
            _ => {

            },
        };

        
        
    }


}

struct Server {
    listener:UnixListener,
}

#[derive(Debug)]
struct Connection {
    stream :BufWriter<UnixStream>,
    buffer:BytesMut,
}

pub enum Message {
    StringMsg(String),
    ByteMsg(Bytes),
    RouteMsg(Bytes),
    NULL,
}
pub type TheError = Box<dyn std::error::Error + Send + Sync>;

impl Message {
    pub fn parse(src:&mut Cursor<&[u8]>) -> Result<Message,TheError> {
        match type_num(src).unwrap() {
            5_u16 => {
                let b_len = src.get_u32() as usize;
                let start = src.position() as usize;
                let line = src.get_ref()[start..(start + b_len)].to_vec();
                Ok(Message::StringMsg(String::from_utf8(line).unwrap()))
            }
            _ => Ok(Message::NULL),
        }
    }


}

pub fn type_num(src:&mut Cursor<&[u8]>) -> Result<u16,TheError> {
    if !src.has_remaining() {
        return Ok(0_u16)
    }
    Ok(src.get_u16())
}
// pub type Result<T> = std::result::Result<T, TheError>;
impl Connection {
    pub fn new(socket: UnixStream) ->Connection {
        Connection { 
            stream: BufWriter::new(socket), 
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }
    fn parse_msg(&mut self) -> Result<Message,TheError> {
        let mut buf = Cursor::new(&self.buffer[..]);
        let msg = Message::parse(&mut buf)?;
        Ok(msg)
    }
    pub async fn read_message(&mut self) -> Result<Message,TheError> {
        loop {
            match self.parse_msg() {
                msg => return Ok(msg.unwrap()),
                _ => (),
            }
            return Ok(Message::NULL);
        }
    }
}
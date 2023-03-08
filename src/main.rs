use std::{io::{self, Write, Read}, collections::HashMap, path::Path, fs, error::Error, sync::Arc};

use bytes::{BytesMut, Buf};
use mio::{Poll, Events, net::{UnixListener, UnixStream}, Token, Interest,Registry};
use mio::event::Event;
use tokio_util::codec::{Decoder, Framed, Encoder};
use std::str::from_utf8;
use tokio_stream::StreamExt;
use futures::SinkExt;
use tokio::sync::{mpsc, Mutex};
use std::net::SocketAddr;
type Tx = mpsc::UnboundedSender<String>;

type Rx = mpsc::UnboundedReceiver<String>;

struct Shared {
    peers:HashMap<String,Tx>,
}
struct Peer {
    lines:Framed<tokio::net::UnixStream,Pong>,
    rx:Rx,
}

impl Shared {
    fn new() ->Self {
        Shared { peers: HashMap::new() }
    }

    async fn broadcast(&mut self,sender:String,message:&String) {
        for peer in self.peers.iter_mut()  {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }
    async fn send_to(&mut self,to:String,msg:&String) {
        let mut peer = self.peers.get(&to);
        
        let mut peer = peer.unwrap();
        peer.send(msg.into());
    }
}

impl Peer {
    async fn new(
        state:Arc<Mutex<Shared>>,
        lines:Framed<tokio::net::UnixStream,Pong>,
        cname:String,
    ) -> io::Result<Peer> {
        let addr = cname;
        let (tx,rx) = mpsc::unbounded_channel();

        state.lock().await.peers.insert(addr, tx);
        Ok(Peer { lines: lines, rx: rx })
    }
}




#[tokio::main]
async fn main(){
    // let mut channelMap:HashMap<String,&mut Framed<tokio::net::UnixStream,Pong>> = HashMap::new();
    let addr_path = Path::new("/tmp/uds.sock");
    if addr_path.exists() {
        fs::remove_file("/tmp/uds.sock");
    }
    // private final short GET_NUM = 1;
    // private final short COMMAND_NUM = 2;
    // private final short BYTE_VALUE_NUM = 3;
    // private final short ROUTE_VALUE_NUM = 4;
    // private final short REG_NUM = 5;

    // let mut channelMap = HashMap::new();
    let state = Arc::new(Mutex::new(Shared::new()));

    let listener = tokio::net::UnixListener::bind("/tmp/uds.sock").unwrap();
    loop {

        match listener.accept().await {
            Ok((stream, _addr)) => {
                println!("new client {:?} !",_addr);


                let mut transport =  Framed::new(stream,Pong);
            
                
                while let Some(msg) = transport.next().await {
                    
                    let res = msg.unwrap();
                    match res.1 {
                        1 =>{
                            println!("get num {}",res.0);
                        },
                        2 =>{
                            println!("command {}",res.0);
                        },
                        3=>{
                            println!("value {}",res.0);
                        },
                        4=>{
                            println!("route {}",res.0);
                        },
                        5=>{
                            // channelMap.insert(res.0, transport.get_ref());
                            println!("reg");
                            let state = Arc::clone(&state);
                            let mut peer = Peer::new(state.clone(), transport,res.0).await;
                            peer.unwrap();
                            // channelMap.insert(res.0, );
                        }
                        _ =>{
                            println!("{}",&res.0);
                        }
                    }
                    transport.send(String::from("value")).await;
                }
            }
            Err(e) => { 
                /* connection failed */
            
            }
        }
    } 
}


const SERVER: Token = Token(0);
// Some data we'll send over the connection.
const DATA: &[u8] = b"Hello world!\n";

fn main_low()-> io::Result<()>{
    
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    let addr = "/tmp/uds.sock";

    let addr_path = Path::new(addr);
    if addr_path.exists() {
        fs::remove_file(addr_path);
    }
    

    let mut server = UnixListener::bind(addr)?;
    poll.registry()
    .register(&mut server, SERVER, Interest::READABLE)?;

    let mut connections = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);
    println!("conn");
    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // If we get a `WouldBlock` error we know our
                            // listener has no more incoming connections queued,
                            // so we can return to polling and wait for some
                            // more.
                            break;
                        }
                        Err(e) => {
                            // If it was any other kind of error, something went
                            // wrong and we terminate with an error.
                            return Err(e);
                        }
                    };

                    println!("Accepted connection from: {:?}", address);

                    let token = next(&mut unique_token);
                    poll.registry().register(
                        &mut connection,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    connections.insert(token, connection);
                },
                token => {
                    // Maybe received an event for a TCP connection.
                    let done = if let Some(connection) = connections.get_mut(&token) {
                        handle_connection_event(poll.registry(), connection, event)?
                    } else {
                        // Sporadic events happen, we can safely ignore them.
                        false
                    };
                    if done {
                        if let Some(mut connection) = connections.remove(&token) {
                            poll.registry().deregister(&mut connection)?;
                        }
                    }
                }
            }
        }
    }
}

/// Returns `true` if the connection is done.
fn handle_connection_event(
    registry: &Registry,
    connection: &mut UnixStream,
    event: &Event,
) -> io::Result<bool> {
    if event.is_writable() {
        
        // We can (maybe) write to the connection.
        match connection.write(DATA) {
            // We want to write the entire `DATA` buffer in a single go. If we
            // write less we'll return a short write error (same as
            // `io::Write::write_all` does).
            Ok(n) if n < DATA.len() => return Err(io::ErrorKind::WriteZero.into()),
            Ok(_) => {
                // After we've written something we'll reregister the connection
                // to only respond to readable events.
                registry.reregister(connection, event.token(), Interest::READABLE)?
            }
            // Would block "errors" are the OS's way of saying that the
            // connection is not actually ready to perform this I/O operation.
            Err(ref err) if would_block(err) => {}
            // Got interrupted (how rude!), we'll try again.
            Err(ref err) if interrupted(err) => {
                return handle_connection_event(registry, connection, event)
            }
            // Other errors we'll consider fatal.
            Err(err) => return Err(err),
        }
    }

    if event.is_readable() {
        // let mut transport = Framed::new(connection,Pong);
        let mut connection_closed = false;
        let mut received_data = vec![0; 4096];
        let mut bytes_read = 0;
        // We can (maybe) read from the connection.
        loop {
            
            match connection.read(&mut received_data[bytes_read..]) {
                Ok(0) => {
                    // Reading 0 bytes means the other side has closed the
                    // connection or is done writing, then so are we.
                    connection_closed = true;
                    break;
                }
                Ok(n) => {
                    bytes_read += n;
                    if bytes_read == received_data.len() {
                        received_data.resize(received_data.len() + 1024, 0);
                    }
                }
                // Would block "errors" are the OS's way of saying that the
                // connection is not actually ready to perform this I/O operation.
                Err(ref err) if would_block(err) => break,
                Err(ref err) if interrupted(err) => continue,
                // Other errors we'll consider fatal.
                Err(err) => return Err(err),
            }
        }

        if bytes_read != 0 {
            let received_data = &received_data[..bytes_read];
            if let Ok(str_buf) = from_utf8(received_data) {
                println!("Received data: {}", str_buf.trim_end());
            } else {
                println!("Received (none UTF-8) data: {:?}", received_data);
            }
        }

        if connection_closed {
            println!("Connection closed");
            return Ok(true);
        }
    }

    Ok(false)
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}

fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}


struct Pong;

impl Decoder for Pong {
    type Item = (String,u16);

    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 1 {
            return Ok(None);
        }
        let flag = src.get_u16();
        let body_len = src.get_u32();
        let str_buf = src.copy_to_bytes(body_len as usize);
        src.advance(2);

        let ss = String::from_utf8_lossy(str_buf.chunk());
        
        

        // let mut buffer = String::new();
        // src.reader().read_to_string(&mut buffer).unwrap();
        // println!("len {}",src.len());
        // let arr = src.get(0..src.len());
        // println!("len {}",src.len());


        Ok(Some((ss.into_owned(),flag)))
        // Ok(None)
        // print!(">:{}",s.to_string());
        // todo!()
    }
}

impl  Encoder<String> for Pong {
    type Error= io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use std::fmt::Write;
        // todo!()
        write!(
            BytesWrite(dst),
            "{}",
            item
        ).unwrap();
        return Ok(());
        struct BytesWrite<'a>(&'a mut BytesMut);

        impl std::fmt::Write for BytesWrite<'_> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0.extend_from_slice(s.as_bytes());
                Ok(())
            }

            fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
                std::fmt::write(self, args)
            }
        }
    }
    
}
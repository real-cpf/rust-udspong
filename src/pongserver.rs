use bytes::Buf;
use bytes::{BufMut, Bytes, BytesMut};
use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::io;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::{
    net::{UnixListener, UnixStream},
};
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use tokio_util::codec::Framed;

struct Shared {
    peers: HashMap<String, Tx>,
    db: HashMap<String,Bytes>,
}

type Tx = mpsc::UnboundedSender<String>;

type Rx = mpsc::UnboundedReceiver<String>;

struct PongPeer {
    lines: Framed<UnixStream, PongCodec>,
    rx: Rx,
}

enum PongMessage {
    GetMsg(String),
    RouteMsg((String,Bytes)),
    DictMsg((String,Bytes)),
    Command(String),
    Reg(String),
    Next,
    None,
}

// GET_NUM         = 1
// COMMAND_NUM     = 2
// BYTE_VALUE_NUM  = 3
// ROUTE_VALUE_NUM = 4
// REG_NUM         = 5

impl PongMessage {
    fn dict(key:String,value:Bytes) -> Result<PongMessage,PongCodecError> {
        Ok(PongMessage::DictMsg((key,value)))
    }
    fn of(t: u16, src:Bytes) -> Result<PongMessage,PongCodecError> {
        match t {
            1_u16 =>{
                let s = String::from_utf8(src.to_vec()).unwrap();
                return Ok(PongMessage::GetMsg(s));
            },
            2_u16 => {
                let s = String::from_utf8(src.to_vec()).unwrap();
                return Ok(PongMessage::Command(s));
            },
            4_u16 => {
                for i in 0..src.len() {
                    if 32_u8 == *src.get(i).unwrap() {
                        let cc = &src.chunk()[0..i];
                        let channel_name = String::from_utf8(cc.to_vec()).unwrap();
                        let mm = src.chunk()[(i+1)..src.len()].to_vec();
                        return Ok(PongMessage::RouteMsg((channel_name,Bytes::from(mm))));
                    }
                }
                return Ok(PongMessage::None);
            },
            5_u16 =>{
                let s = String::from_utf8(src.to_vec()).unwrap();
                return Ok(PongMessage::Reg(s));
            },
            _ =>{
                return Ok(PongMessage::None);
            }
        }
    }
}

impl PongPeer {
    async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<UnixStream, PongCodec>,
        addr: String,
    ) -> io::Result<PongPeer> {
        let (tx, rx) = mpsc::unbounded_channel();
        state.lock().await.peers.insert(addr, tx);
        let merge :[Bytes;2]= [Bytes::new(),Bytes::new()];
        Ok(PongPeer { lines, rx})
    }
}

impl Shared {
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
            db: HashMap::new(),
        }
    }
    async fn do_put(&mut self,key:String,value:Bytes) {
        println!("put key{}",key.clone());
        self.db.insert(key,value);
    }
    async fn do_get(&mut self,key:String)->Option<&Bytes> {
        self.db.get(&key)
    }
    async fn do_boardcast(&mut self,sender:String,message:&str) {
        
        self.peers.iter_mut().for_each(|peer|{
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        });
    }
    async fn do_route(&mut self, sender: String, message: &str) {
        let rx = self.peers.get_mut(&sender).unwrap();
        let res = rx.send(message.into());
        match res {
            Err(e) =>{
                println!("rx.send error {:?}",e.0);
            }
            Ok(_) =>{
                println!("rx.send ok");
            }
        }
        println!("rx.send:{}",message.clone());

    }
}

// GET_NUM         = 1
// COMMAND_NUM     = 2
// BYTE_VALUE_NUM  = 3
// ROUTE_VALUE_NUM = 4
// REG_NUM         = 5
async fn handle_stream(state: Arc<Mutex<Shared>>, stream: UnixStream) -> Result<(), Box<dyn Error>> {
    let mut lines = Framed::new(stream, PongCodec::new());

    let entry = match lines.next().await {
        Some(Ok(line)) => line,
        _ => {
            return Ok(());
        }
    };
    let channel_name = match entry {
        PongMessage::Reg(ref r) => r.to_string(),
        _=>String::from("")
    };
    // let channel_name = String::from_utf8(entry.1.to_vec()).unwrap();
    let mut peer = PongPeer::new(state.clone(), lines, channel_name.clone())
        .await
        .unwrap();
    {
        let mut state = state.lock().await;
        let msg = format!("{} reg done !", channel_name);
        println!("reg");
        state.do_route(channel_name.clone(), &msg).await;
        // state.do_put(key, value)
    }
    
    // let buff = Bytes::new();
    // String::from_utf8(buff.chunk().to_vec()).unwrap();
    // peer.msg_merge[peer.merge_index] = Bytes::new();
    
    loop {
        tokio::select! {
            Some(msg) = peer.rx.recv() => {
                let buf = bytes::Bytes::from(msg.clone());
                println!("send:{}",msg);
                peer.lines.send(buf).await.unwrap();
            }
            result = peer.lines.next() => match result {
                Some(Ok(msg)) => {
                    match msg {
                        PongMessage::GetMsg(ref g) => {
                            let mut state = state.lock().await;
                            let vv = state.do_get(g.to_string()).await.unwrap();
                            let mut res = Bytes::new();
                            res.clone_from(vv);
                            peer.lines.send(res).await.unwrap();
                        },
                        PongMessage::DictMsg((ref k,ref v)) =>{
                            let mut state = state.lock().await;
                            state.do_put(k.to_string(), v.clone()).await;
                            peer.lines.send(bytes::Bytes::from("ok")).await.unwrap();
                        },
                        PongMessage::RouteMsg((ref c,ref v)) => {
                            let mut state = state.lock().await;
                            let s = String::from_utf8(v.to_vec()).unwrap();
                            state.do_route(c.to_string(),&format!("{}",s)).await;
                            peer.lines.send(bytes::Bytes::from("ok")).await.unwrap();
                        },
                        _ =>{
                            println!("none");
                        }
                    }
                }
                // An error occurred.
                Some(Err(e)) => {
                    println!("{:?}",e)
                }
                // The stream has been exhausted.
                None => break,
            },
        }
        println!("out select");
    }
    {
        let mut state = state.lock().await;
        let msg = format!("{} unreg",channel_name.clone());
        state.do_route(channel_name.clone(), &msg).await;
        state.peers.remove(&channel_name);

    }
    println!("disconnect!");
    Ok(())
}

/// An error occurred while encoding or decoding a line.
#[derive(Debug)]
pub enum PongCodecError {
    EndOfBuf,
    WrongTypeNum,
    /// An IO error occurred.
    Io(io::Error),
}

impl std::fmt::Display for PongCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PongCodecError::EndOfBuf => write!(f, "end of buf"),
            PongCodecError::WrongTypeNum => write!(f, "write num"),
            PongCodecError::Io(e) => write!(f, "{}", e),
        }
    }
}
impl From<io::Error> for PongCodecError {
    fn from(e: io::Error) -> PongCodecError {
        PongCodecError::Io(e)
    }
}

impl std::error::Error for PongCodecError {}

struct PongCodec {
    last_num:u16,
}
impl PongCodec {
    fn new()->PongCodec {
        PongCodec { last_num: 0 }
    }
}
impl Decoder for PongCodec {
    type Item = PongMessage;

    type Error = PongCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        let num = match self.last_num  {
            0 => src.get_u16(),
            _=> self.last_num
        };
        if check_type_num(num) {
            let body_len = src.get_u32() as usize;
            let body = src.copy_to_bytes(body_len);
            src.advance(2);
            if src.has_remaining() {
                self.last_num = src.get_u16();
            }
            // TODO the src.get_u16 will advance 2 ,next will be error shuoud use the decode.num to record last num?
            if num == 3_u16 && self.last_num == 3_u16 {
                let body_len = src.get_u32() as usize;
                let body_value = src.copy_to_bytes(body_len);
                src.advance(2);
                self.last_num = 0;
                return Ok(Some(PongMessage::dict(String::from_utf8(body.to_vec()).unwrap(), body_value).unwrap()));
            }
            Ok(Some(PongMessage::of(num, body).unwrap()))
        } else {
            Err(PongCodecError::WrongTypeNum)
        }
    }
}

impl Encoder<Bytes> for PongCodec {
    type Error = PongCodecError;

    fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.reserve(data.len());
        buf.put(data);
        Ok(())
    }
}
impl Encoder<BytesMut> for PongCodec {
    type Error = PongCodecError;

    fn encode(&mut self, data: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(data.len());

        dst.put(data);
        Ok(())
    }
}

fn check_type_num(n: u16) -> bool {
    if n > 0 && n < 6 {
        true
    } else {
        false
    }
}
/*
write client code
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = "/tmp/uds.sock";
    let path_addr = Path::new(bind_addr);
    if path_addr.exists() {
        std::fs::remove_file(bind_addr).unwrap();
    }
    let state = Arc::new(Mutex::new(Shared::new()));
    let listener = UnixListener::bind(bind_addr).unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_stream(state, stream).await {
                println!("{:?}", e);
            }
        });
    }
}

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Cursor;
use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use futures::SinkExt;
use futures::StreamExt;
use futures::lock::Mutex;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::{
    io::BufWriter,
    net::{UnixListener, UnixStream},
};
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;


struct Shared {
    peers: HashMap<String, Tx>,
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<String>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<String>;


struct Peer {
    lines : Framed<UnixStream,LinesCodec>,
    rx:Rx,
}

impl Shared {
    fn new()->Self{
        Shared { peers: HashMap::new() }
    }
    async fn broadcast(&mut self,sender:String,message:&str) {
        self.peers.iter_mut().for_each(|peer|{
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        });
    }
}

impl Peer {
    async fn new(
        state:Arc<Mutex<Shared>>,
        lines:Framed<UnixStream,LinesCodec>,
        addr:String,
    ) -> io::Result<Peer> {
        let (tx,rx) = mpsc::unbounded_channel();
        state.lock().await.peers.insert(addr, tx);
        Ok(Peer { lines, rx })
    }
}

async fn handler(
    state:Arc<Mutex<Shared>>,
    stream:UnixStream,
) -> Result<(),Box<dyn Error>>{
    let mut lines = Framed::new(stream,LinesCodec::new());
    lines.send("enter your name:").await.unwrap();
    let addr = match lines.next().await {
        Some(Ok(line)) => line,
        _ =>{
            // String::from("defalut");
            return Ok(());
        }
    };
    let mut peer = Peer::new(state.clone(),lines,addr.clone()).await.unwrap();
    {
        let mut state = state.lock().await;
        let msg = format!("{} joined !",addr);
        state.broadcast(addr.clone(), &msg).await;
    }
    loop {
        tokio::select! {
            Some(msg) = peer.rx.recv() => {
                peer.lines.send(msg).await.unwrap();
            }
            result = peer.lines.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", addr.clone(), msg);

                    state.broadcast(addr.clone(), &msg).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    println!("{:?}",e)
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    Ok(())
}


#[tokio::main]
async fn main() -> Result<(),Box<dyn Error>> {
    let state = Arc::new(Mutex::new(Shared::new()));
    let listener = UnixListener::bind("/tmp/uds.sock").unwrap();


    loop {
        let (stream,_) = listener.accept().await.unwrap();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) =  handler(state, stream).await{
                println!("{:?}",e);
            }
        });
    }
}

#[tokio::main]
async fn main_simaple() {
    let listener = UnixListener::bind("/tmp/uds.sock").unwrap();

    let mut channel_map:HashMap<String, UnixStream> = HashMap::new();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            loop {
                let mut buf = BytesMut::with_capacity(1024);
                if 0 == socket.read_buf(&mut buf).await.unwrap() {
                    break;
                }
                let len = buf.len();
                let arr = buf.to_vec();
                let s = String::from_utf8(arr).unwrap();
                let ss:Vec<&str> = s.split(' ').collect();
                
                println!("key {}",ss[0]);
                socket.write_all(ss[1].as_bytes()).await;
                
            }
        });
        println!("done this accept!");
    }
}

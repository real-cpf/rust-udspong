use std::error::Error;


use bytes::{BytesMut, BufMut};
use tokio::{net::UnixStream, io::{BufWriter, AsyncReadExt, AsyncWriteExt}};



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = "/tmp/uds.sock";
    let stream = UnixStream::connect(bind_addr).await.unwrap();
    let stream = BufWriter::new(stream);
    let mut reg_msg = BytesMut::with_capacity(10);
    reg_msg.put_u16(5_u16);
    reg_msg.put_u32(2_u32);
    reg_msg.put_u8(b'c');
    reg_msg.put_u8(b'1');
    reg_msg.put_u8(b'\r');
    reg_msg.put_u8(b'\n');
    loop{}
    
}
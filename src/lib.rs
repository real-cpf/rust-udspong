pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixStream, io::{Write, Bytes, Read}};

    use super::*;

    
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
    #[test]
    fn test_send(){
        let mut stream = UnixStream::connect("/tmp/uds.sock").unwrap();
        let  buf:[u8;10] = [0,1,0,0,0,2,99,49,13,10];
        stream.write_all(&buf).unwrap();
        let mut res:[u8;1024]=[0;1024];
        stream.read(&mut res).unwrap();
        let s = String::from_utf8(res.to_vec()).unwrap();
        println!("res:{}",&s);
        
    }
}

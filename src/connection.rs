use bytes::Buf;
use bytes::BytesMut;
use mini_redis::frame::Error::Incomplete;
use mini_redis::{Frame, Result};
use std::io::Cursor;
use tokio::io::BufWriter;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Connection {
    // stream: TcpStream,
    // buffer: Vec<u8>,
    // cursor: usize,
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            // stream,
            // buffer: vec![0; 4096],
            // cursor: 0,
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4096),
        }
    }
}

pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
    loop {
        if let Some(frame) = self.parse_frame()? {
            return Ok(Some(frame));
        }

        if self.buffer.len() == self.cursor {
            self.buffer.resize(self.cursor * 2, 0);
        }

        let n = self.stream.read(&mut self.buffer[self.cursor..]).await?;

        if 0 == n {
            if self.cursor == 0 {
                return Ok(None);
            } else {
                return Err("connection reset by peer".into());
            }
        } else {
            self.cursor += n;
        }
    }
}

async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
    match Frame {
        Frame::Simple(val) => {
            self.stream.write_u8(b'+').await?;
            self.stream.write_all(val.as_bytes()).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Error(val) => {
            self.stream.write_u8(b"-").await?;
            self.stream.write_all(val.as_bytes()).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Integer(val) => {
            self.stream.write_u8(b":").await?;
            self.write_decimal(*val).await?;
        }
        Frame::Null => {
            self.stream.write_all(b"$-1\r\n").await?;
        }
        Frame::Bulk() => {
            let len = val.len();
            self.stream.write_u8(b"$").await?;
            self.write_decimal(len as u64).await?;
            self.stream.write_all(val).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Array(_val) => unimplemented!(),
    }

    self.stream.flush().await;
    Ok(())
}

fn parse_frame(&mut self) -> Result<Option<Frame>> {
    let mut buf = Cursor::new(&self.buffer[..]);

    match Frame::check(&mut buf) {
        Ok(_) => {
            let len = buf.position() as usize;
            buf.set_position(0);
            let frame = Frame::parse(&mut buf)?;
            self.buffer.advance(len);
            Ok(Some(buffer))
        }
        Err(Incomplete) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

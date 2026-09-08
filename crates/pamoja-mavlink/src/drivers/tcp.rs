//! A TCP [`ByteLink`], the transport ArduPilot SITL exposes by default (port 5760).
//!
//! ArduPilot's SITL listens for a ground station on TCP, and MAVLink then rides the stream as a
//! continuous byte flow the [`Parser`](crate::Parser) frames. [`TcpLink`] connects to such a
//! server; [`from_stream`](TcpLink::from_stream) wraps an already-accepted stream, so a server
//! side can use the same driver.

use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

use super::link_fault;
use crate::error::Result;
use crate::link::ByteLink;

/// A TCP stream presented as a [`ByteLink`].
pub struct TcpLink {
    stream: TcpStream,
}

impl TcpLink {
    /// Connects to a MAVLink TCP server, such as ArduPilot SITL on `127.0.0.1:5760`.
    ///
    /// Nagle's algorithm is disabled so a frame is sent without waiting to coalesce, which keeps
    /// control latency low.
    ///
    /// # Arguments
    ///
    /// * `addr` - the server address to connect to.
    ///
    /// # Returns
    ///
    /// The connected link.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the connection cannot be made.
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let _ = stream.set_nodelay(true);
        Ok(TcpLink { stream })
    }

    /// Wraps an already-connected stream, for the accepting side of a connection.
    ///
    /// # Arguments
    ///
    /// * `stream` - the connected TCP stream.
    ///
    /// # Returns
    ///
    /// The link over the stream.
    pub fn from_stream(stream: TcpStream) -> Self {
        let _ = stream.set_nodelay(true);
        TcpLink { stream }
    }
}

impl ByteLink for TcpLink {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.stream.read(buf).await.map_err(link_fault)
    }

    async fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await.map_err(link_fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{CommandAck, CommandLong, Message};
    use crate::link::Connection;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn a_command_round_trips_over_a_real_tcp_connection() {
        // A listener stands in for a MAVLink TCP server (as ArduPilot SITL is); a command and
        // its acknowledgment cross a real localhost TCP connection through the driver.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut vehicle = Connection::new(TcpLink::from_stream(stream), 1, 1);
            let frame = vehicle.recv().await.unwrap();
            let command = CommandLong::decode(frame.payload()).unwrap();
            let ack = CommandAck {
                command: command.command,
                result: 0,
                progress: 0,
                result_param2: 0,
                target_system: frame.system_id(),
                target_component: frame.component_id(),
            };
            vehicle.send(&ack).await.unwrap();
        });

        let mut gcs = Connection::new(TcpLink::connect(addr).await.unwrap(), 255, 190);
        let arm = CommandLong {
            param1: 1.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            param7: 0.0,
            command: 400,
            target_system: 1,
            target_component: 1,
            confirmation: 0,
        };
        gcs.send(&arm).await.unwrap();
        let frame = gcs.recv().await.unwrap();
        assert_eq!(frame.message_id(), CommandAck::ID);
        server.await.unwrap();
    }
}

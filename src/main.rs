use std::time::Duration;
use async_std::future::timeout;
use async_std::net::TcpStream;
use async_std::io::{WriteExt, ReadExt};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use sha2::{Digest, Sha256}; 
use rand;

const DASH_MAINNET_MAGIC: [u8; 4] = [0xbf, 0x0c, 0x6b, 0xbd];
const DASH_DEFAULT_PORT: &str = "9999";
const COMMAND_VERSION: &str = "version";
const COMMAND_VERACK: &str = "verack";

fn calculate_checksum(payload: &[u8]) -> Vec<u8> {
    let hash1 = Sha256::digest(payload);
    let hash2 = Sha256::digest(&hash1);
    hash2[0..4].to_vec()
}

fn create_version_message_payload() -> Vec<u8> {
    let mut payload = Vec::new();

    // protocol version
    payload.write_i32::<LittleEndian>(70230).unwrap();

    // services
    payload.write_u64::<LittleEndian>(0x00).unwrap();

    // timestamp
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    payload.write_i64::<LittleEndian>(timestamp.as_secs() as i64).unwrap();

    // addr_recv services
    payload.write_u64::<LittleEndian>(0x01).unwrap();

    // addr_recv IP address
    payload.extend(&[0, 0, 0, 0, 0, 0, 0xff, 0xff]); // IPv4-mapped IPv6 address prefix
    payload.extend(&[8,219,5,90]); // Masternode's IPv4 part of the mapped address

    // addr_recv port
    payload.write_u16::<BigEndian>(9999).unwrap();

    // addr_trans services
    payload.write_u64::<LittleEndian>(0x00).unwrap();

    // addr_trans IP address
    payload.extend(&[0; 16]); // IPv6 unspecified address

    // addr_trans port
    payload.write_u16::<BigEndian>(9999).unwrap();

    // nonce
    let nonce = rand::random::<u64>();
    payload.write_u64::<LittleEndian>(nonce).unwrap();

    // user agent
    let user_agent = b"/DashRustClient:0.1.0/";
    payload.write_all(&[(user_agent.len() as u8)]);  // bytes
    payload.write_all(user_agent);  // string

    // start height
    payload.write_i32::<LittleEndian>(0).unwrap();

    payload
}

fn create_message(command: &str, payload: &[u8]) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend(&DASH_MAINNET_MAGIC);

    // Add command name
    message.extend(command.as_bytes());

    // Calculate padding length and add padding
    let padding_length = 12 - command.len();
    let padding = vec![0u8; padding_length];
    message.extend(&padding);

    // Add payload length and checksum
    message.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
    let checksum = calculate_checksum(payload);
    message.extend(&checksum);

    // Add payload
    message.extend(payload);

    message
}

fn get_command_from_response(response: &[u8]) -> String {
    let end_index = usize::min(response.len(), 16);
    String::from_utf8_lossy(&response[4..end_index]).trim_end_matches('\u{0}').to_string()
}

async fn read_with_timeout(
    stream: &mut TcpStream, 
    buffer: &mut [u8], 
    timeout_duration: Duration,
) -> std::io::Result<usize> {
    timeout(timeout_duration, async {
        stream.read(buffer).await
    })
    .await
    .map_or(Ok(0), |result| result)
}

async fn perform_handshake() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting handshake...");

    let mut stream = TcpStream::connect(format!("8.219.5.90:{}", DASH_DEFAULT_PORT)).await?;
    println!("Connected to node.");

    // Send version message
    let version_message = create_message(COMMAND_VERSION, &create_version_message_payload());
    println!("Sending version message to node...");
    stream.write_all(&version_message).await?;

    let mut received_version = false;
    let mut handshake_complete = false;

    while !handshake_complete {
        let mut response = vec![0u8; 1024];
        let read_size = read_with_timeout(&mut stream, &mut response, Duration::from_secs(10)).await?;

        if read_size == 0 {
            break;
        }

        let command = get_command_from_response(&response[..read_size]);
        match command.as_str() {
            "version" if !received_version => {
                println!("Received version message from peer");
                received_version = true;

                let verack_message = create_message(COMMAND_VERACK, &[]);
                println!("Sending verack message to peer...");
                stream.write_all(&verack_message).await?;
            },
            "verack" if received_version => {
                println!("Received verack response");
                handshake_complete = true;
            },
            "inv" if received_version => {
                println!("Received inv message");
                handshake_complete = true;
            },
            _ => println!("Received {} response", command),
        }
    }

    if handshake_complete {
        println!("Handshake completed successfully");
    } else {
        println!("Handshake did not complete fully");
    }

    Ok(())
}

fn main() {
    async_std::task::block_on(perform_handshake()).unwrap();
}

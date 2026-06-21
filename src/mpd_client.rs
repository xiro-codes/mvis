use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub file: String,
}

pub struct MpdClient {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl MpdClient {
    pub fn connect(host: &str) -> Option<Self> {
        let stream = TcpStream::connect(host).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
        stream.set_write_timeout(Some(Duration::from_secs(2))).ok()?;
        
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        
        // Read greeting "OK MPD <version>\n"
        let mut greeting = String::new();
        reader.read_line(&mut greeting).ok()?;
        if !greeting.starts_with("OK MPD") {
            return None;
        }

        Some(Self { stream, reader })
    }

    pub fn get_current_song(&mut self) -> Option<SongInfo> {
        self.stream.write_all(b"currentsong\n").ok()?;
        
        let mut title = String::new();
        let mut artist = String::new();
        let mut file = String::new();

        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line).is_err() || line.is_empty() {
                return None;
            }
            if line.starts_with("OK") {
                break;
            }
            if line.starts_with("ACK") {
                return None;
            }

            if let Some(stripped) = line.strip_prefix("Title: ") {
                title = stripped.trim().to_string();
            } else if let Some(stripped) = line.strip_prefix("Artist: ") {
                artist = stripped.trim().to_string();
            } else if let Some(stripped) = line.strip_prefix("file: ") {
                file = stripped.trim().to_string();
            }
        }

        if file.is_empty() {
            return None;
        }

        Some(SongInfo { title, artist, file })
    }

    pub fn get_album_art(&mut self, uri: &str) -> Option<Vec<u8>> {
        let cmd = format!("readpicture \"{}\" 0\n", uri);
        self.stream.write_all(cmd.as_bytes()).ok()?;
        
        let mut binary_size = 0;
        
        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line).is_err() || line.is_empty() {
                return None;
            }
            if line.starts_with("OK") {
                return None; // No picture found
            }
            if line.starts_with("ACK") {
                return None;
            }
            if let Some(size_str) = line.strip_prefix("binary: ") {
                binary_size = size_str.trim().parse::<usize>().unwrap_or(0);
                break;
            }
        }

        if binary_size == 0 {
            return None;
        }

        let mut img_data = vec![0u8; binary_size];
        self.reader.read_exact(&mut img_data).ok()?;

        // clear out remaining lines up to OK
        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line).is_err() || line.is_empty() {
                break;
            }
            if line.starts_with("OK") || line.starts_with("ACK") {
                break;
            }
        }

        Some(img_data)
    }
}

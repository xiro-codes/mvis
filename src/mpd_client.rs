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
        // Increased timeout to 10 seconds to handle large album art chunks
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .ok()?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .ok()?;

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

        Some(SongInfo {
            title,
            artist,
            file,
        })
    }

    pub fn get_status(&mut self) -> Option<(f32, f32)> {
        self.stream.write_all(b"status\n").ok()?;

        let mut elapsed = 0.0;
        let mut duration = 0.0;

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

            if let Some(stripped) = line.strip_prefix("elapsed: ") {
                elapsed = stripped.trim().parse().unwrap_or(0.0);
            } else if let Some(stripped) = line.strip_prefix("duration: ") {
                duration = stripped.trim().parse().unwrap_or(0.0);
            } else if let Some(stripped) = line.strip_prefix("time: ") {
                let parts: Vec<&str> = stripped.trim().split(':').collect();
                if parts.len() == 2 && elapsed == 0.0 && duration == 0.0 {
                    elapsed = parts[0].parse().unwrap_or(0.0);
                    duration = parts[1].parse().unwrap_or(0.0);
                }
            }
        }

        Some((elapsed, duration))
    }

    fn fetch_picture(&mut self, cmd_name: &str, uri: &str) -> Option<Vec<u8>> {
        let mut offset = 0;
        let mut total_size = None;
        let mut data = Vec::new();

        let safe_uri = uri.replace("\"", "\\\"");

        loop {
            let cmd = format!("{} \"{}\" {}\n", cmd_name, safe_uri, offset);
            if self.stream.write_all(cmd.as_bytes()).is_err() {
                return None;
            }

            let mut chunk_size = 0;

            loop {
                let mut line = String::new();
                if self.reader.read_line(&mut line).is_err() || line.is_empty() {
                    return if data.is_empty() { None } else { Some(data) };
                }
                if line.starts_with("OK") {
                    if chunk_size == 0 {
                        return if data.is_empty() { None } else { Some(data) };
                    }
                    break;
                }
                if line.starts_with("ACK") {
                    return if data.is_empty() { None } else { Some(data) };
                }

                if let Some(size_str) = line.strip_prefix("size: ") {
                    total_size = Some(size_str.trim().parse::<usize>().unwrap_or(0));
                } else if let Some(size_str) = line.strip_prefix("binary: ") {
                    chunk_size = size_str.trim().parse::<usize>().unwrap_or(0);
                    break;
                }
            }

            if chunk_size == 0 {
                break;
            }

            let mut chunk_data = vec![0u8; chunk_size];
            if self.reader.read_exact(&mut chunk_data).is_err() {
                break;
            }
            data.extend_from_slice(&chunk_data);

            // Read until we see "OK"
            loop {
                let mut line = String::new();
                if self.reader.read_line(&mut line).is_err() || line.is_empty() {
                    break;
                }
                if line.starts_with("OK") || line.starts_with("ACK") {
                    break;
                }
            }

            offset += chunk_size;

            if let Some(ts) = total_size {
                if offset >= ts {
                    break;
                }
            } else {
                // If total_size wasn't provided, just break (fallback)
                break;
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(data)
        }
    }

    pub fn get_album_art(&mut self, uri: &str) -> Option<Vec<u8>> {
        if let Some(data) = self.fetch_picture("readpicture", uri) {
            return Some(data);
        }
        self.fetch_picture("albumart", uri)
    }
}

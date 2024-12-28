use clap::{Parser, Subcommand};
use image::GenericImageView;
use regex::Regex;
use std::fs::File;
use std::io::Cursor;
use std::io::{BufReader, Read};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "StegHtml")]
#[command(version = "1.0")]
#[command(author = "Mycoearthdome")]
#[command(about = "Steganography over HTML")]
struct Cli {
    /// The mode of operation
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    /// Normal mode for hiding data in an image
    Start {
        /// The input filename containing the data to be hidden
        #[arg(short, long, value_name = "FILE")]
        input_filename: String,

        /// The image filename where the data will be hidden
        #[arg(short, long, value_name = "IMAGE")]
        filename_image: String,

        /// The server address in the format ip:port for communication
        #[arg(short, long, value_name = "ADDRESS")]
        server_address: String,

        /// Optional width of the output image (default: 0)
        #[arg(short, long, default_value_t = 1080, value_name = "WIDTH")]
        width: u32,

        /// Optional height of the output image (default: 0)
        #[arg(short, long, default_value_t = 2244, value_name = "HEIGHT")]
        height: u32,
    },

    /// Proxy mode for handling requests
    Proxy {
        /// The server address in the format ip:port for the proxy
        #[arg(short, long, value_name = "ADDRESS")]
        server_address: String,

        /// The listening port for the proxy
        #[arg(short, long, value_name = "PORT")]
        listening_port: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Match on the mode to determine the operation
    match cli.mode {
        Mode::Proxy {
            server_address,
            listening_port,
        } => {
            println!("Running in proxy mode");
            println!("Server Address: {}", server_address);
            println!("Listening Port: {}", listening_port);

            let server_address = server_address;
            let server = tokio::net::TcpListener::bind(&format!("127.0.0.1:{}", listening_port))
                .await
                .unwrap();
            loop {
                // listen
                let mut receive_server: [u8; 65535] = [0; 65535];
                let mut receive_client: Vec<u8> = Vec::new();
                let (mut server_stream, _socks) = server.accept().await.unwrap();
                let n = server_stream.read(&mut receive_server).await.unwrap();
                // connect the request to server
                let mut client_stream = TcpStream::connect(&server_address).await.unwrap();
                client_stream.write_all(&receive_server[..n]).await.unwrap();

                let _n = client_stream
                    .read_to_end(&mut receive_client)
                    .await
                    .unwrap();
                let header = String::from_utf8(receive_client[..80].to_vec()).unwrap();

                let re = Regex::new(r"Content-Length: (\d+)").unwrap();

                if let Some(captures) = re.captures(&header) {
                    if let Some(content_length) = captures.get(1) {
                        println!("Content-Length: {}", content_length.as_str());
                        let payload_index = header.find("Content-Length: ").unwrap()
                            + 20
                            + content_length.as_str().len();
                        let content_length = content_length.as_str().parse().unwrap();
                        let payload = receive_client[payload_index..].to_vec();

                        let message =
                            decode_payload(payload, content_length).unwrap();

                        let null_bytes_free_message = message.trim_end_matches('\0').to_string();

                        let response = [format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", null_bytes_free_message.len()).as_bytes(),null_bytes_free_message.as_bytes(), "\0".as_bytes()].concat();

                        server_stream.write_all(&response).await.unwrap();
                    }
                } else {
                    println!("{}", header);
                    server_stream.write_all(&receive_client).await.unwrap();
                }
            }
        }
        Mode::Start {
            input_filename,
            filename_image,
            server_address,
            mut width,
            mut height,
        } => {
            println!("Running in normal mode");

            let background_filename = filename_image;
            let server_address = server_address;
            let server_ip: Vec<_> = server_address.split(":").collect();
            let server_ip = server_ip[0];
            // Create a new image with the same dimensions as the body element
            if width == 0 {
                width = 1080; // replace with your width
            } else {
                width = width;
            }
            if height == 0 {
                height = 2244; // replace with your height
            } else {
                height = height;
            }

            let request = format!(
                "GET / HTTP/1.1\r\n\
                           Host: {}\r\n\
                           Connection: close\r\n\
                           \r\n",
                server_ip
            );

            println!("Reading the bits...");
            // Read the bit stream ile
            let bits = read_bit_stream(&input_filename);

            println!("Modyfying the background...");
            // Modify the background color pixel by pixel
            let mut bit_index = 0;
            let mut new_background = image::RgbaImage::new(width, height);
            for x in 0..width {
                for y in 0..height {
                    if bit_index >= bits.len() {
                        // Default to original pixels if the bit stream has ended
                        let pixel = image::Rgba([255, 255, 255, 255]);
                        new_background.put_pixel(x, y, pixel);
                    } else {
                        let mut new_pixel = [255, 255, 255, 255];
                        for i in 0..4 {
                            let bit = bits[bit_index];
                            if bit == true {
                                new_pixel[i] = new_pixel[i] - 1;
                            }
                            bit_index += 1;
                        }
                        new_background.put_pixel(x, y, image::Rgba(new_pixel));
                    }
                }
            }

            // save the modded background
            let _ = image::save_buffer_with_format(
                &background_filename.clone(),
                &new_background,
                width,
                height,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            );
            // Serve the modified image
            println!("Serving with 127.0.0.1:8080");
            let server = tokio::net::TcpListener::bind("127.0.0.1:8080")
                .await
                .unwrap();
            
            let re = Regex::new(r#"<a\s+href=["']?([^"'>]+)["']?>(.*?)</a>"#).unwrap();
            loop {
                let mut receive_server: [u8; 65535] = [0; 65535];
                let mut receive_client: Vec<u8> = Vec::new();
                let (mut server_stream, _socks) = server.accept().await.unwrap();
                let n = server_stream.read(&mut receive_server).await.unwrap();
                let server_text = String::from_utf8(receive_server[..n].to_vec()).unwrap();
                if server_text.contains("/////") {
                    let mut background_file = File::open(&background_filename).unwrap();
                    let mut file_bytes = Vec::new();
                    background_file.read_to_end(&mut file_bytes).unwrap();
                    let response = [format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", file_bytes.len()).as_bytes(),file_bytes.as_slice()].concat();
                    let _ = server_stream.write_all(&response).await;
                    continue;
                }
                let mut client_stream = TcpStream::connect(&server_address).await.unwrap();
                client_stream
                    .write_all(request.to_string().as_bytes())
                    .await
                    .unwrap();
                let n = client_stream
                    .read_to_end(&mut receive_client)
                    .await
                    .unwrap();
                let client_text = String::from_utf8(receive_client[..n].to_vec()).unwrap();
                let start_html = client_text.find("<html>").unwrap();
                let end_html = client_text.find("</html>").unwrap();
                let mut client_text = client_text[start_html..end_html + 7].to_string();
                let mut modified_text = client_text.clone();

                // Find all matches
                for capture in re.captures_iter(&client_text) {
                    // Capture group 1 contains the URL
                    if let Some(_url) = capture.get(1) {
                        // Capture group 2 contains the label
                        if let Some(label) = capture.get(2) {
                            let new_link = if label.as_str() == "../" {
                                format!("<a href=\"././/\">{}</a>", label.as_str())
                            } else {
                                format!(
                                    "<a href=\"http://{}/{}\">{}</a>",
                                    server_address,
                                    label.as_str(),
                                    label.as_str()
                                )
                            };
                            if label.as_str() == "../" {
                                modified_text = client_text.replace(
                                    &format!(
                                        "<a href=\"{}\">{}</a>",
                                        label.as_str(),
                                        label.as_str()
                                    ),
                                    &new_link,
                                );
                            } else {
                                modified_text = modified_text.replace(
                                    &format!(
                                        "<a href=\"{}\">{}</a>",
                                        label.as_str(),
                                        label.as_str()
                                    ),
                                    &new_link,
                                );
                            }
                        }
                    }
                }

                client_text = modified_text.replace("<body>", "");
                client_text = client_text.replace("<head><title>Index of /</title></head>", "");

                let response = "HTTP/1.1 200 OK\r\nContent-Type text/html\r\n";
                let html_doc = format!(
                    "{}",
                    String::from_utf8(
                        [response.as_bytes(), client_text.as_bytes()]
                            .concat()
                            .to_vec()
                    )
                    .unwrap()
                );
                let response = &html_doc;
                println!("{}", response);
                server_stream.write_all(response.as_bytes()).await.unwrap();
            }
        }
    }

    fn bools_to_utf8_string(bools: Vec<bool>) -> String {
        let mut bytes = Vec::new();

        // Process the boolean vector in chunks of 8
        for chunk in bools.chunks(8) {
            let mut byte = 0u8; // Initialize a byte to 0
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << i; // Set the corresponding bit if true
                }
            }
            bytes.push(byte); // Add the byte to the vector
        }

        // Convert the byte vector to a UTF-8 string
        String::from_utf8(bytes).unwrap_or_else(|_| String::from("Invalid UTF-8"))
    }

    fn decode_payload(payload: Vec<u8>, content_length: u32) -> Result<String, String> {
        // Create a Cursor from the payload
        let cursor = Cursor::new(payload);
        let mut bitstream: Vec<bool> = Vec::new();
        let mut actual_bytes_count = 0;
        // Attempt to load the image
        match image::load(cursor, image::ImageFormat::Png) {
            Ok(img) => {
                // You can access the image dimensions if needed
                let (img_width, img_height) = img.dimensions();
                for x in 0..img_width as u32 {
                    for y in 0..img_height as u32 {
                        if actual_bytes_count <= content_length {
                            let pixel = img.get_pixel(x, y);
                            for i in 0..4 {
                                if pixel[i] == 255 {
                                    bitstream.push(false);
                                } else {
                                    bitstream.push(true);
                                }
                            }
                        }
                        actual_bytes_count += 1;
                    }
                }
                // println!("actual bytes count = {}", actual_bytes_count);
                // println!("content_length = {}", content_length);
                // take the bitstream to bytes
                let message = bools_to_utf8_string(bitstream);
                println!("{}", message);

                Ok(message)
            }
            Err(e) => Err(format!("Failed to load image: {}", e)),
        }
    }
}

fn read_bit_stream(file_path: &str) -> Vec<bool> {
    let mut bit_stream = Vec::new();
    if let Ok(file) = File::open(file_path) {
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).unwrap_or(0);

        // Convert each byte into bits
        for byte in buffer {
            for bit in 0..8 {
                //(0..8).rev() { // uncomment if you work with windows.
                bit_stream.push((byte & (1 << bit)) != 0);
            }
        }
    }
    bit_stream
}

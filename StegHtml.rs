use tokio;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::fs::File;
use std::io::{BufReader, Read};
use tokio::net::TcpStream;
use regex::Regex;


#[tokio::main]
async fn main() {

    let background_filename = "background.png";
    let server_address = "148.113.201.63:80"; // Change to your desired server
    let server_ip: Vec<_> = server_address.split(":").collect();
    let server_ip = server_ip[0];
    // Create a new image with the same dimensions as the body element
    let width = 1080; // replace with your width
    let height = 2244; // replace with your height
    let request = format!("GET / HTTP/1.1\r\n\
                   Host: {}\r\n\
                   Connection: close\r\n\
                   \r\n", server_ip);
    
    println!("Reading the bits...");
    // Read the bit stream ile
    let bits = read_bit_stream("Spirit-of-iron.txt");

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
    let _ = image::save_buffer_with_format(background_filename, &new_background, width, height, image::ColorType::Rgba8, image::ImageFormat::Png);
    // Serve the modified image
    println!("Serving...Modded html");
    let server = tokio::net::TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let re = Regex::new(r#"<a\s+href=["']?([^"'>]+)["']?>(.*?)</a>"#).unwrap();
    loop{
        let mut receive_server:[u8;65535] = [0;65535];
        let mut receive_client:[u8;65535] = [0;65535];
        let (mut server_stream, _socks) = server.accept().await.unwrap();
        let n = server_stream.read(&mut receive_server).await.unwrap();
        let server_text = String::from_utf8(receive_server[..n].to_vec()).unwrap();
        if server_text.contains("/////"){
            let mut background_file = File::open(background_filename).unwrap();
            let mut file_bytes = Vec::new();
            background_file.read_to_end(&mut file_bytes).unwrap();
            let response = [format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", file_bytes.len()).as_bytes(),file_bytes.as_slice()].concat();
            let _ = server_stream.write_all(&response).await;
            continue;
        }
        let mut client_stream = TcpStream::connect(server_address).await.unwrap();
        client_stream.write_all(request.to_string().as_bytes()).await.unwrap();
        let n = client_stream.read(&mut receive_client).await.unwrap();
        let client_text = String::from_utf8(receive_client[..n].to_vec()).unwrap();
        let start_html = client_text.find("<html>").unwrap();
        let end_html = client_text.find("</html>").unwrap();
        let mut client_text = client_text[start_html..end_html+7].to_string();
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
                        format!("<a href=\"http://{}/{}\">{}</a>", server_address, label.as_str(), label.as_str())
                    };
                    if label.as_str() == "../"{
                        modified_text = client_text.replace(&format!("<a href=\"{}\">{}</a>", label.as_str(), label.as_str()), &new_link);
                    } else {
                        modified_text = modified_text.replace(&format!("<a href=\"{}\">{}</a>", label.as_str(), label.as_str()), &new_link);
                    }
                }
            }
        }

        client_text = modified_text.replace("<body>", "");
        client_text = client_text.replace("<head><title>Index of /</title></head>", "");

        let response = "HTTP/1.1 200 OK\r\nContent-Type text/html\r\n";
        let html_doc = format!("{}", String::from_utf8([response.as_bytes(), client_text.as_bytes()].concat().to_vec()).unwrap());
        let response = &html_doc;
        println!("{}", response);
        server_stream.write_all(response.as_bytes()).await.unwrap();
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
            for bit in (0..8){ //.rev() { // uncomment if you work with windows.
                bit_stream.push((byte & (1 << bit)) != 0);
            }
        }
    }
    bit_stream
}
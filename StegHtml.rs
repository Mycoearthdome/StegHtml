use tokio;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::fs::File;
use std::io::{BufReader, Read};
use tokio::net::TcpStream;


#[tokio::main]
async fn main() {

    // Create a new image with the same dimensions as the body element
    let width = 1080; // replace with your width
    let height = 2244; // replace with your height
    let request = "GET / HTTP/1.1\r\n\
                   Host: 148.113.201.63\r\n\
                   Connection: close\r\n\
                   \r\n";

    println!("Reading the bits...");
    // Read the bit stream ile
    let bits = read_bit_stream("Spirit-of-iron.txt");

    println!("Modyfying the background in html...");
    // Modify the background color pixel by pixel
    let mut bit_index = 0;
    let mut html = String::new();
    for x in 0..width {
        for y in 0..height {
            if bit_index >= bits.len() {
                // Default to original pixels if the bit stream has ended
                let original_img = image::open("original_image.png").unwrap().to_rgba8();
                let pixel = original_img.get_pixel(x, y);
                html.push_str(&format!("<span style='position: absolute; left: {}px; top: {}px; width: 1px; height: 1px; background-color: rgba({}, {}, {}, {});'></span>", x, y, pixel[0], pixel[1], pixel[2], pixel[3]));
            } else {
                let bit = bits[bit_index];
                let original_img = image::open("original_image.png").unwrap().to_rgba8();
                let pixel = original_img.get_pixel(x, y);
                let mut new_pixel = [0; 4];
                for i in 0..4 {
                    if bit == true {
                        new_pixel[i] = pixel[i] - 1;
                    }
                }
                html.push_str(&format!("<span style='position: absolute; left: {}px; top: {}px; width: 1px; height: 1px; background-color: rgba({}, {}, {}, {});'></span>", x, y, new_pixel[0], new_pixel[1], new_pixel[2], new_pixel[3]));
                bit_index += 1;
            }
        }
    }

    // Serve the modified image
    println!("Serving...Modded html");
    let server = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let server_address = "148.113.201.63"; // Change to your desired server
    let mut receive:[u8;65535] = [0;65535];
    loop{
        let (mut server_stream, _socks) = server.accept().await.unwrap();
        let mut client_stream = TcpStream::connect(server_address).await.unwrap();
        client_stream.write_all(request.to_string().as_bytes()).await.unwrap();
        let n = client_stream.read(&mut receive).await.unwrap();
        let client_text = String::from_utf8(receive[..n].to_vec()).unwrap();
        let start_html = client_text.find("<HTML>").unwrap();
        let end_html = client_text.find("</HTML>").unwrap();
        let client_text = &client_text[start_html..end_html+5];
        let client_text = client_text.replace("<a href=\"", &format!{"<a href=\"{}/", server_address});
        
        let response = "HTTP/1.1 200 OK\nContent-Type text/html\n";
        let html_doc = format!("<html><body style='width: {}px; height: {}px; position: relative;'>{}{}</body></html>", width, height, client_text, html);
        let response = response.to_owned() + &html_doc;
        server_stream.write_all(response.to_string().as_bytes()).await.unwrap();
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
            for bit in (0..8).rev() { // Extract bits from most significant to least significant
                bit_stream.push((byte & (1 << bit)) != 0);
            }
        }
    }
    bit_stream
}
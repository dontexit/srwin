use http::Response as httpResponse;
use serde_json::{Result, Value};
use std::{
    collections::HashMap,
    fmt, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
    u16,
};
fn main() {
    println!("Server? Is this the server?");
    let addr = "127.0.1";
    let port = "6969";
    let listener = TcpListener::bind(format!("{addr}:{port}")).unwrap();
    println!("Started server on {addr}:{port}");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

#[derive(Debug)]
#[repr(u16)]
enum StatusCode {
    Ok = 200,
    NotFound = 404,
    InternalError = 500,
    BadRequest = 400,
    Other(u16),
}

impl StatusCode {
    fn status(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::NotFound => "Not Found",
            StatusCode::InternalError => "Internal Server Error",
            _ => " ",
        }
    }
    fn from_u16(code: u16) -> Self {
        match code {
            200 => StatusCode::Ok,
            404 => StatusCode::NotFound,
            500 => StatusCode::InternalError,
            400 => StatusCode::BadRequest,
            _ => StatusCode::Other(code),
        }
    }
}

impl Into<StatusCode> for u16 {
    fn into(self) -> StatusCode {
        StatusCode::from_u16(self)
    }
}

struct Response {
    code: u16,
    status: &'static str,
    body: Body,
    headers: Option<HashMap<String, String>>,
}

enum Body {
    File(String),
    String(String),
    None,
}
impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Body::File(file_name) => write!(f, "{}", file_name),
            Body::String(body_string) => write!(f, "{}", body_string),
            Body::None => write!(f, ""),
        }
    }
}
impl Default for Response {
    fn default() -> Self {
        Response {
            code: 200,
            status: StatusCode::Ok.status(),
            body: Body::String(String::new()),
            headers: None,
        }
    }
}
impl Response {
    fn new() -> Self {
        Response {
            code: 200,
            status: StatusCode::Ok.status(),
            body: Body::String(String::new()),
            headers: None,
        }
    }

    fn code(mut self, code: u16) -> Self {
        let code_var: StatusCode = code.into();
        self.code = code;
        self.status = code_var.status();
        self
    }

    fn headers(mut self, headers: Option<HashMap<String, String>>) -> Self {
        self.headers = headers;
        self
    }

    fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    fn as_resp(self) -> String {
        let content = match self.body {
            Body::File(filename) => match fs::File::open(&filename) {
                Ok(file) => {
                    let mut file_buf = BufReader::new(file);
                    let mut contents = String::new();
                    if let Err(err) = file_buf.read_to_string(&mut contents) {
                        eprint!("{}", err);
                        None
                    } else {
                        Some(contents)
                    }
                }
                Err(err) => {
                    eprint!("Error reading file: {}", err);
                    None
                }
            },
            Body::String(contents) => Some(contents),
            Body::None => None,
        };

        let content_length = match &content {
            Some(value) => value.len(),
            None => 0,
        };
        // self.body.len();
        print!(" {}\n", &self.code);
        format!(
            "{HTTP_VER} {}{HEADER}{CONTENT_LENGTH}{content_length}{BODY}{}",
            self.code,
            content.unwrap_or("".to_string())
        )
    }

    fn render(self, mut stream: TcpStream) {
        stream
            .write_all(&self.as_resp().into_bytes())
            .unwrap_or_else(|err| {
                eprintln!("Error: {}", err);
            });
        stream.flush().unwrap_or_else(|err| {
            println!("flush failed with error: {}", err);
        });
    }
}

fn render(mut stream: TcpStream, res: Response) {
    stream
        .write_all(&res.as_resp().into_bytes())
        .unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
        });
    stream.flush().unwrap_or_else(|err| {
        println!("flush failed with error: {}", err);
    });
}

const HEADER: &'static str = "\r\n";
const BODY: &'static str = "\r\n\r\n";

#[derive(Debug)]
enum HttpVersion {
    V1,
    V2,
    V3,
}

// impl HttpVersion {
//     fn version(&self) -> &'static str {
//         todo!()
//     }
// }
const HTTP_VER: &'static str = "HTTP/1.1";
const CONTENT_LENGTH: &'static str = "Content-Length:";
// enum Request {
//
// }
// fn parse_req(mut stream: TcpStream) -> Request{
//     todo!()
// }
//
//
fn write_stream_or_log(mut stream: TcpStream, res: Response) {
    stream
        .write_all(&res.as_resp().into_bytes())
        .unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
        });
    stream.flush().unwrap_or_else(|err| {
        println!("flush failed with error: {}", err);
    });
}

fn handle_connection(mut stream: TcpStream) {
    let mut stream_buf: BufReader<_> = BufReader::new(&stream);
    let mut req_vec: Vec<String> = Vec::new();
    for stream_line in stream_buf.lines() {
        match stream_line {
            Ok(line) => {
                if line.trim().is_empty() {
                    break;
                }
                // println!("{}", &line);
                req_vec.push(line.to_owned());
            }
            Err(err) => {
                println!("Error reading stream: {}", err);
                break;
            }
        }
    }

    let first_line = req_vec.get(0);
    let (method, uri, http_ver) = match first_line {
        Some(val) => {
            print!("{}", &val);
            let vals: Vec<_> = val.split(" ").collect();
            (
                vals.get(0).map(|s| *s),
                vals.get(1).map(|s| *s),
                vals.get(2).map(|s| *s),
            )
        }
        None => (None, None, None),
    };

    let req = Request {
        method: RequestMethod::from_str(method).unwrap_or(GET),
        headers: None,
    };

    match uri {
        Some(val) => match val {
            "/" => index(req).render(stream),
            "/sleep" | "/sleep/" => {
                thread::sleep(Duration::from_secs(5));
                Response::new().body(Body::String(String::from(
                    "Just woke up from sleep, dont mind me :)",
                )));
            }
            &_ => default_404(req).render(stream),
        },
        None => {
            println!("On None");
            Response::new()
                .code(404)
                .body(Body::File(String::from("404.html")))
                .render(stream)
        }
    };
}

struct Request {
    method: RequestMethod,
    headers: Option<HashMap<String, String>>,
}

enum RequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}
use RequestMethod::*;

impl RequestMethod {
    fn from_str(method: Option<&str>) -> Option<Self> {
        match method {
            Some(value) => match value.to_lowercase().as_str() {
                "get" => Some(RequestMethod::GET),
                "post" => Some(RequestMethod::POST),
                "put" => Some(RequestMethod::PUT),
                "patch" => Some(RequestMethod::PATCH),
                "delete" => Some(RequestMethod::DELETE),
                _ => None,
            },
            None => None,
        }
    }
}

fn test(req: Request) -> Response {
    match req.method {
        RequestMethod::GET => Response::new().body(Body::File(String::from("test.html"))),
        _ => Response::new().code(405),
    }
}

fn index(req: Request) -> Response {
    match req.method {
        GET => Response::new().body(Body::File(String::from("hello.html"))),
        _ => Response::new().code(405),
    }
}
fn default_404(req: Request) -> Response {
    match req.method {
        GET => Response::new()
            .code(404)
            .body(Body::File(String::from("404.html"))),
        _ => Response::new().code(405),
    }
}

use std::fmt::Display;
use std::net::SocketAddr;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Request, Response, Body, Method};
use hyper::body::aggregate;
use hyper::body::Buf;
use tokio::net::TcpListener;
use tokio::select;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::{self, Duration};
use serde::{Deserialize, Serialize};
use std::fmt::{Formatter, Result as FmtResult};
use serde::ser::StdError;
use serde_json::from_reader;

#[derive(Debug)]
pub enum EigenError {
	ConnectionError,
	ListenError,
	AggregateError,
	ParseError,
	InvalidQuery,
	InvalidRequest,
}

impl Display for EigenError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		write!(f, "{:?}", self)?;
		Ok(())
	}
}

impl StdError for EigenError {}

#[derive(Serialize, Deserialize, Debug)]
struct SignatureData {
	pubkey: u8,
	sig_r_x: u8,
	sig_r_y: u8,
	sig_s: u8,
}

async fn hello(req: Request<Body>) -> Result<Response<String>, EigenError> {
	match (req.method(), req.uri().path()) {
		(&Method::GET, "/score") => {
			let q = req.uri().query();
			let query_string = q.ok_or(EigenError::InvalidQuery)?;
			let pair: Vec<&str> = query_string.split("=").collect();
			match pair[..] {
				["pubkey", value] => {
					println!("required pubkey score {:?}", value);
				},
				_ => return Err(EigenError::InvalidQuery),
			}
		},
		(&Method::POST, "/signature") => {
			// Aggregate the body...
			let whole_body = aggregate(req).await.map_err(|_| EigenError::AggregateError)?;
			// Decode as JSON...
			let data: SignatureData = from_reader(whole_body.reader()).map_err(|_| EigenError::ParseError)?;
			println!("posted signature {:?}", data);
		},
		_ => return Err(EigenError::InvalidRequest),
	}
	Ok(Response::new(String::from("Hello World!")))
}

async fn handle_connection<I: AsyncRead + AsyncWrite + Unpin + 'static>(stream: I, _addr: SocketAddr) {
	let mut https = Http::new();
	https.http1_keep_alive(false);
	let res = https.serve_connection(stream, service_fn(hello)).await;
	if let Err(err) = res {
		println!("Error serving connection: {:?}", err);
	}
}

#[tokio::main]
pub async fn main() -> Result<(), EigenError> {
	let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

	let listener = TcpListener::bind(addr).await.map_err(|_| EigenError::ListenError)?;
	println!("Listening on https://{}", addr);

	let interval = Duration::from_secs(2);
	let mut inner_interval = time::interval(interval);
	inner_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

	loop {
		select! {
			res = listener.accept() => {
				let (stream, addr) = res.map_err(|_| EigenError::ConnectionError)?;
				handle_connection(stream, addr).await;
			}
			res = inner_interval.tick() => {
				println!("{:?}", res);
			}
		};
	}
}
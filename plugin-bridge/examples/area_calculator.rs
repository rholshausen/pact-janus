//! The area calculator from pact-plugins' gRPC example (`examples/gRPC/area_calculator`, MIT): a gRPC
//! provider that answers a shape, or several, with their areas. Staged plan S5.2e verifies its consumer's
//! pact against it through the bridge's `grpc` transport, and S1.1's harness against today's verifier
//! (`corpora/verifier/plugins/grpc-area-calculator*`).
//!
//! The messages are the example's `area_calculator.proto`, written as prost would generate them, and the
//! service is written as tonic would generate its server, so building it needs no `protoc`.
//!
//! ```text
//! area_calculator [--port <port>] [--answers areas|nothing|error]
//! ```
//!
//! It listens on 127.0.0.1, port 0 unless told, and prints `listening on <port>` once it is. `--answers`
//! is what it answers with: the areas (the default), an `AreaResponse` with none, or a gRPC `INTERNAL`
//! status. It exits when its standard input ends, so a harness that dies leaves nothing running.

use std::convert::Infallible;
use std::io::{Read, Write};
use std::sync::Arc;
use std::task::{Context, Poll};
use tonic::body::Body;
use tonic::codegen::{BoxFuture, Service, http};
use tonic::server::{Grpc, NamedService, UnaryService};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, prost::Message)]
pub struct ShapeMessage {
  #[prost(oneof = "shape_message::Shape", tags = "1, 2, 3, 4, 5")]
  pub shape: Option<shape_message::Shape>,
}

pub mod shape_message {
  #[derive(Clone, PartialEq, prost::Oneof)]
  pub enum Shape {
    #[prost(message, tag = "1")]
    Square(super::Square),
    #[prost(message, tag = "2")]
    Rectangle(super::Rectangle),
    #[prost(message, tag = "3")]
    Circle(super::Circle),
    #[prost(message, tag = "4")]
    Triangle(super::Triangle),
    #[prost(message, tag = "5")]
    Parallelogram(super::Parallelogram),
  }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Square {
  #[prost(float, tag = "1")]
  pub edge_length: f32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Rectangle {
  #[prost(float, tag = "1")]
  pub length: f32,
  #[prost(float, tag = "2")]
  pub width: f32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Circle {
  #[prost(float, tag = "1")]
  pub radius: f32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Triangle {
  #[prost(float, tag = "1")]
  pub edge_a: f32,
  #[prost(float, tag = "2")]
  pub edge_b: f32,
  #[prost(float, tag = "3")]
  pub edge_c: f32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Parallelogram {
  #[prost(float, tag = "1")]
  pub base_length: f32,
  #[prost(float, tag = "2")]
  pub height: f32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AreaRequest {
  #[prost(message, repeated, tag = "1")]
  pub shapes: Vec<ShapeMessage>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AreaResponse {
  #[prost(float, repeated, tag = "1")]
  pub value: Vec<f32>,
}

/// What the provider answers with.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Answers {
  Areas,
  Nothing,
  Error,
}

/// One shape's area, as the example's providers compute it.
fn area(shape: &ShapeMessage) -> Result<f32, Status> {
  use shape_message::Shape;
  match &shape.shape {
    Some(Shape::Square(square)) => Ok(square.edge_length * square.edge_length),
    Some(Shape::Rectangle(rectangle)) => Ok(rectangle.length * rectangle.width),
    Some(Shape::Circle(circle)) => Ok(std::f32::consts::PI * circle.radius * circle.radius),
    Some(Shape::Triangle(triangle)) => {
      let p = (triangle.edge_a + triangle.edge_b + triangle.edge_c) / 2.0;
      Ok((p * (p - triangle.edge_a) * (p - triangle.edge_b) * (p - triangle.edge_c)).sqrt())
    }
    Some(Shape::Parallelogram(parallelogram)) => Ok(parallelogram.base_length * parallelogram.height),
    None => Err(Status::invalid_argument("no shape was given")),
  }
}

fn answer(answers: Answers, shapes: &[ShapeMessage]) -> Result<Response<AreaResponse>, Status> {
  match answers {
    Answers::Areas => Ok(Response::new(AreaResponse {
      value: shapes.iter().map(area).collect::<Result<_, _>>()?,
    })),
    Answers::Nothing => Ok(Response::new(AreaResponse::default())),
    Answers::Error => Err(Status::internal("the area calculator is out of order")),
  }
}

/// `area_calculator.Calculator`, as tonic generates a server for it.
#[derive(Clone)]
struct CalculatorServer {
  answers: Answers,
}

impl NamedService for CalculatorServer {
  const NAME: &'static str = "area_calculator.Calculator";
}

struct CalculateOne(Answers);

impl UnaryService<ShapeMessage> for CalculateOne {
  type Response = AreaResponse;
  type Future = BoxFuture<Response<AreaResponse>, Status>;

  fn call(&mut self, request: Request<ShapeMessage>) -> Self::Future {
    let answers = self.0;
    Box::pin(async move { answer(answers, std::slice::from_ref(request.get_ref())) })
  }
}

struct CalculateMulti(Answers);

impl UnaryService<AreaRequest> for CalculateMulti {
  type Response = AreaResponse;
  type Future = BoxFuture<Response<AreaResponse>, Status>;

  fn call(&mut self, request: Request<AreaRequest>) -> Self::Future {
    let answers = self.0;
    Box::pin(async move { answer(answers, &request.get_ref().shapes) })
  }
}

impl<B> Service<http::Request<B>> for CalculatorServer
where
  B: tonic::codegen::Body + Send + 'static,
  B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
  type Response = http::Response<Body>;
  type Error = Infallible;
  type Future = BoxFuture<Self::Response, Self::Error>;

  fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn call(&mut self, request: http::Request<B>) -> Self::Future {
    let answers = self.answers;
    match request.uri().path() {
      "/area_calculator.Calculator/calculateOne" => Box::pin(async move {
        let mut grpc = Grpc::new(tonic_prost::ProstCodec::default());
        Ok(grpc.unary(CalculateOne(answers), request).await)
      }),
      "/area_calculator.Calculator/calculateMulti" => Box::pin(async move {
        let mut grpc = Grpc::new(tonic_prost::ProstCodec::default());
        Ok(grpc.unary(CalculateMulti(answers), request).await)
      }),
      _ => Box::pin(async move { Ok(Status::unimplemented("no such method").into_http()) }),
    }
  }
}

fn usage(message: &str) -> ! {
  eprintln!(
    "area_calculator: {message}\nusage: area_calculator [--port <port>] [--answers areas|nothing|error]"
  );
  std::process::exit(2);
}

fn main() {
  let mut port = 0u16;
  let mut answers = Answers::Areas;
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--port" => {
        port = args
          .next()
          .and_then(|port| port.parse().ok())
          .unwrap_or_else(|| usage("--port takes a port number"))
      }
      "--answers" => {
        answers = match args.next().as_deref() {
          Some("areas") => Answers::Areas,
          Some("nothing") => Answers::Nothing,
          Some("error") => Answers::Error,
          _ => usage("--answers takes areas, nothing or error"),
        }
      }
      other => usage(&format!("unknown argument '{other}'")),
    }
  }

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap_or_else(|err| usage(&format!("could not start a runtime: {err}")));
  runtime.block_on(async move {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
      .await
      .unwrap_or_else(|err| usage(&format!("could not listen on port {port}: {err}")));
    let bound = listener.local_addr().map(|addr| addr.port()).unwrap_or(port);
    println!("listening on {bound}");
    let _ = std::io::stdout().flush();

    // Standard input is its lease on life, as a component's is.
    let (done, ended) = tokio::sync::oneshot::channel::<()>();
    let done = Arc::new(std::sync::Mutex::new(Some(done)));
    std::thread::spawn(move || {
      let mut sink = Vec::new();
      let _ = std::io::stdin().read_to_end(&mut sink);
      if let Some(done) = done.lock().ok().and_then(|mut done| done.take()) {
        let _ = done.send(());
      }
    });

    let served = tonic::transport::Server::builder()
      .add_service(CalculatorServer { answers })
      .serve_with_incoming_shutdown(tokio_stream::wrappers::TcpListenerStream::new(listener), async {
        let _ = ended.await;
      })
      .await;
    if let Err(err) = served {
      eprintln!("area_calculator: {err}");
      std::process::exit(1);
    }
  });
}

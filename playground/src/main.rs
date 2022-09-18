use anyhow::Result;
use futures::Stream;
use pb::{
    math_request::Op,
    playground_server::{Playground, PlaygroundServer},
    CountdownRequest, CountdownResponse, HangmanRequest, HangmanResponse, HashRequest,
    HashResponse, MathRequest, MathResponse, SecretRequest, SecretResponse,
};
use std::{boxed::Box, pin::Pin};
use tonic::{transport::Server, Request, Response, Status, Streaming};

mod pb {
    tonic::include_proto!("playground");
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Default)]
struct PlaygroundImpl;

#[tonic::async_trait]
impl Playground for PlaygroundImpl {
    async fn math(&self, req: Request<MathRequest>) -> Result<Response<MathResponse>, Status> {
        let (lhs, rhs) = (req.get_ref().lhs, req.get_ref().rhs);
        let result = match req.get_ref().op() {
            Op::Add => lhs + rhs,
            Op::Subtract => lhs - rhs,
            Op::Multiply => lhs * rhs,
            Op::Divide => lhs / rhs,
        };
        Ok(Response::new(MathResponse { result }))
    }

    type CountdownStream = ResponseStream<CountdownResponse>;
    async fn countdown(
        &self,
        _req: Request<CountdownRequest>,
    ) -> Result<Response<Self::CountdownStream>, Status> {
        todo!()
    }

    async fn hash(
        &self,
        _req: Request<Streaming<HashRequest>>,
    ) -> Result<Response<HashResponse>, Status> {
        todo!()
    }

    type HangmanStream = ResponseStream<HangmanResponse>;
    async fn hangman(
        &self,
        _req: Request<Streaming<HangmanRequest>>,
    ) -> Result<Response<Self::HangmanStream>, Status> {
        todo!()
    }

    async fn secret(
        &self,
        _req: Request<SecretRequest>,
    ) -> Result<Response<SecretResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:7575".parse()?;
    let pg = PlaygroundImpl::default();

    println!("Listening on {}", &addr);

    Server::builder()
        .add_service(PlaygroundServer::new(pg))
        .serve(addr)
        .await?;
    Ok(())
}

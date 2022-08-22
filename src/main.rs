use std::{io, sync::mpsc};
use serde::{Deserialize};
use actix_web::{web, App, HttpResponse, HttpServer, middleware, Responder};
use actix_cors::Cors;

use gqlmapi_rs::{MAPIGraphQL};

fn map_recv_error(err: mpsc::RecvError) -> String {
    format!("Error receiving message: {}", err)
}

struct AppState {
    gqlmapi: MAPIGraphQL,
}

#[derive(Deserialize)]
struct GraphQLRequest {
    query: String,
    #[serde(default)] // want to allow queries that omit variables, so make this param optional
    variables: String,
    #[serde(default)] // want to allow queries that omit variables, so make this param optional
    operation_name: String,
}

// Synchronously executes the given GraphQL request against the provided gqlmapi service instance
fn execute_query(gqlmapi: &MAPIGraphQL, request: &GraphQLRequest) -> Result<String, String> {
    let (tx_next, rx_next) = mpsc::channel();
    let (tx_complete, rx_complete) = mpsc::channel();
    
    // Parse the query and create a listener for its results
    let parsed_query = gqlmapi.parse_query(request.query.as_str())?;
    let result_listener = gqlmapi.subscribe(parsed_query, request.operation_name.as_str(), request.variables.as_str());
    let mut result_listener_locked = result_listener.lock().map_err(|err| format!("error code: {}", err))?;

    // Initiate async listening for results of the parsed query
    result_listener_locked.listen(tx_next, tx_complete)?;

    // Block until our query results are ready
    rx_complete.recv().map_err(map_recv_error)?;
    
    rx_next.recv().map_err(map_recv_error)
}

fn execute_graphql_request(gqlmapi: &MAPIGraphQL, request: &GraphQLRequest) -> HttpResponse {
    let result = execute_query(&gqlmapi, &request);
    let response = result.unwrap_or_else(|err| { format!("{{\"errors\":[\"{}\"]}}", &err)});

    HttpResponse::Ok().content_type("application/json").body(response)
}

// graphql route for HTTP GET
async fn graphql_get(app_state: web::Data<AppState>, request: web::Query<GraphQLRequest>) -> impl Responder {
    let request = request.into_inner();
    execute_graphql_request(&(app_state.gqlmapi), &request)
}

// graphql route for HTTP POST
async fn graphql_post(app_state: web::Data<AppState>, request: web::Json<GraphQLRequest>) -> impl Responder {
    execute_graphql_request(&(app_state.gqlmapi), &request)
}

// graphiql route
async fn graphiql() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../public/index.html"))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    print!("Spinning up MAPI GraphQL service...");
    let app_state = web::Data::new(AppState {
        gqlmapi: MAPIGraphQL::new(false /*use_default_profile*/),
    });
    println!("done!");
    
    let host = "localhost";
    let port = 8080;
    let graphql_route_path = "/graphql";
    let graphiql_route_path = "/graphiql";
    print!("Spinning up web server at {host}:{port}...", host=host, port=port);
    let running_server = HttpServer::new(move || {
        let cors_policy = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .send_wildcard();

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors_policy)
            .app_data(app_state.clone())
            .service(web::resource(graphql_route_path)
                .route(web::get().to(graphql_get))
                .route(web::post().to(graphql_post)))
            .route(graphiql_route_path, web::get().to(graphiql))
            
    })
    .bind((host, port))?
    .run();
    println!("done!");

    println!("MAPI GraphQL server up and running at http://{host}:{port}, with these routes:
    \t{graphql_route_path} - HTTP-based GraphQL query execution a la https://graphql.org/learn/serving-over-http
    \t{graphiql_route_path} - GraphiQL IDE for this service to facilitate query crafting/schema exploration");

    running_server.await
}
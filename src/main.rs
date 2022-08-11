use std::{io, sync::mpsc};
use serde::{Serialize, Deserialize};
use actix_web::{web, App, HttpResponse, HttpServer, middleware, Responder};

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

#[derive(Serialize)]
struct GraphQLResponse
{
    data: Option<serde_json::Value>,
    errors: Option<Vec<String>>,
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

fn format_response(result: Result<String, String>) -> GraphQLResponse {
    match result {
        Ok(data) => {
            let data = serde_json::from_str(&data);
            match data {
                Ok(data) => GraphQLResponse { data: Some(data), errors: None },
                Err(err) => GraphQLResponse { data: None, errors: Some(vec![err.to_string()]) },
            }
        },
        Err(err) => GraphQLResponse { data: None, errors: Some(vec![err]) },
    }
}

// graphql route for HTTP GET
async fn graphql_get(app_state: web::Data<AppState>, request: web::Query<GraphQLRequest>) -> impl Responder {
    let request = request.into_inner();
    let result = execute_query(&(app_state.gqlmapi), &request);
    let body = format_response(result);

    HttpResponse::Ok().body(serde_json::to_string(&body).expect("Failed to serialize the GraphQLResponse"))
}

// graphql route for HTTP POST
async fn graphql_post(app_state: web::Data<AppState>, request: web::Json<GraphQLRequest>) -> impl Responder {
    let result = execute_query(&(app_state.gqlmapi), &request);
    let body = format_response(result);

    HttpResponse::Ok().body(serde_json::to_string(&body).expect("Failed to serialize the GraphQLResponse"))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let app_state = web::Data::new(AppState {
        gqlmapi: MAPIGraphQL::new(false /*use_default_profile*/),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            .service(web::resource("/graphql")
                .route(web::get().to(graphql_get))
                .route(web::post().to(graphql_post)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
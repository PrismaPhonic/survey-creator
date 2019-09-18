use actix_web::{middleware, web, App, Error as AWError, HttpResponse, HttpServer, Result, HttpRequest, Either};
use survey_manager_api::commands::{Pool, handle, handle_command_async};
use survey_manager_api::inputs::{CreateSurveyDTO, UpdateSurveyDTO};
use survey_manager_core::app_services::commands::{SurveyCommands, CreateSurveyCommand, UpdateSurveyCommand};
use survey_manager_core::app_services::token::*;
use futures::{IntoFuture, Future};
use serde_derive::{Serialize, Deserialize};
use dotenv::dotenv;
use uuid::Uuid;
use survey_manager_core::app_services::queries::{FindSurveyQuery, FindSurveysByAuthorQuery};
use survey_manager_api::queries::handle_queries_async;
use survey_manager_api::utils::token_from_req;
use actix_web::error::ErrorUnauthorized;
use futures::future::err;
use survey_manager_api::extractors::Token as BearerToken;

const MISSING_TOKEN_STR: &'static str = "You must supply a JWT as a bearer token in the auth headers to access that resource.";

// For grabbing a token from get_token endpoint.
#[derive(Serialize)]
struct Token {
    token: String,
}

#[derive(Deserialize)]
pub struct SurveyId {
    id: String,
}

#[derive(Deserialize)]
pub struct Author {
    author: String,
}

fn create_survey(
    dto: web::Json<CreateSurveyDTO>,
    pool: web::Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = AWError> {
    let create_survey_command: CreateSurveyCommand = dto.into_inner().into();

    handle_command_async(&pool, create_survey_command.into())
        .from_err()
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn update_survey(
    dto: web::Json<UpdateSurveyDTO>,
    pool: web::Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = AWError> {
    let update_survey_command: UpdateSurveyCommand = dto.into_inner().into();

    handle_command_async(&pool, update_survey_command.into())
        .from_err()
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn find_survey(
    pool: web::Data<Pool>,
    token: web::Data<BearerToken>,
    params: web::Path<SurveyId>,
) -> impl Future<Item = HttpResponse, Error = AWError> {
    let Payload{username, ..} = decode_payload(&token.into_inner());

    let find_survey_query = FindSurveyQuery {
        id: params.into_inner().id,
        requesting_author: username,
    };

    handle_queries_async(&pool, find_survey_query.into())
        .from_err()
        .and_then(|res| {
            let text = if let Some(s) = res { s } else { "".to_string() };
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(text))
        })
}

fn find_authors_surveys(
    token: web::Data<BearerToken>,
    pool: web::Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = AWError> {
    let Payload{username, ..} = decode_payload(&token.into_inner());
    let find_authors_surveys = FindSurveysByAuthorQuery { author: username, page_config: None };

    handle_queries_async(&pool, find_authors_surveys.into())
        .from_err()
        .and_then(|res| {
            let text = if let Some(s) = res { s } else { "".to_string() };
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(text))
        })
}

fn get_token(
) -> Result<HttpResponse, AWError> {
    let fake_user_id = Uuid::new_v4();
    let token_str = create_token("test user".to_string(), fake_user_id.to_string());
    let token = Token { token: token_str, };
    Ok(HttpResponse::Ok().json(token))
}

fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = mysql::Pool::new(&database_url).unwrap();

    // Start http server
    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::resource("/survey")
                    .route(web::post().to_async(create_survey))
                    .route(web::patch().to_async(update_survey))
                    .route(web::get().to_async(find_authors_surveys)),
        )
            .service(
                web::resource("/survey/{id}")
                    .route(web::get().to_async(find_survey)),
            )
            .service(
                web::resource("/token")
                    .route(web::get().to(get_token)),
            )
    })
        .bind("127.0.0.1:8080")?
        .run()
}

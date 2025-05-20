use actix_web::{
    error::ErrorInternalServerError,
    web, HttpResponse, Responder, Result as ActixResult,
};
use engine::{execute_workflow, Workflow};
use regex::Regex;
use secrecy::Secret;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use storage::db::{get_logs, insert_log};
use storage::secrets::{get_credential, store_credential};
use uuid::Uuid;

#[derive(Deserialize)]
struct SecretInput {
    name: String,
    token: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg
        .service(web::resource("/workflow").route(web::post().to(handle_workflow)))
        .service(web::resource("/logs").route(web::get().to(get_all_logs)))
        .service(web::resource("/secrets").route(web::post().to(add_secret)))
        .service(web::resource("/secrets/{id}").route(web::get().to(get_secret)));
}

async fn handle_workflow(
    payload: web::Json<Workflow>,
    db: web::Data<PgPool>,
) -> ActixResult<HttpResponse> {
    // 1) Take ownership and resolve any {{secret:UUID}} in headers
    let mut wf = payload.into_inner();
    resolve_secrets(&mut wf, &db).await?;

    // 2) Serialize for logging
    let raw_req = json!(&wf);

    // 3) Execute the workflow
    let result = execute_workflow(wf)
        .await
        .map_err(|e| ErrorInternalServerError(format!("Execution error: {}", e)))?;

    // 4) Serialize response and log
    let raw_res = json!(&result);
    insert_log(&db, raw_req, raw_res)
        .await
        .map_err(|e| ErrorInternalServerError(format!("Log error: {}", e)))?;

    // 5) Return the result
    Ok(HttpResponse::Ok().json(result))
}

async fn get_all_logs(db: web::Data<PgPool>) -> impl Responder {
    match get_logs(&db).await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn add_secret(
    db: web::Data<PgPool>,
    body: web::Json<SecretInput>,
) -> impl Responder {
    match store_credential(&db, &body.name, Secret::new(body.token.clone())).await {
        Ok(id) => HttpResponse::Ok().json(json!({ "id": id })),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get_secret(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    match get_credential(&db, path.into_inner()).await {
        Ok(cred) => HttpResponse::Ok().json(cred),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Scans every header value for any `{{secret:UUID}}` substring and replaces each match
async fn resolve_secrets(
    workflow: &mut Workflow,
    pool: &PgPool,
) -> ActixResult<()> {
    // regex to capture {{secret:UUID}}
    let re = Regex::new(r"\{\{secret:([0-9a-fA-F\-]+)\}\}")
        .map_err(|e| ErrorInternalServerError(format!("Invalid regex: {}", e)))?;

    for step in workflow.steps.iter_mut() {
        if let Some(headers) = &mut step.headers {
            for (_key, value) in headers.iter_mut() {
                let mut new_val = value.clone();
                // for each capture, fetch and decrypt the secret, then replace
                for cap in re.captures_iter(value) {
                    let uuid_str = &cap[1];
                    let id = Uuid::parse_str(uuid_str)
                        .map_err(|e| ErrorInternalServerError(format!("Bad UUID {}: {}", uuid_str, e)))?;
                    let cred = get_credential(pool, id)
                        .await
                        .map_err(|e| ErrorInternalServerError(format!("Secret lookup {}: {}", id, e)))?;
                    new_val = new_val.replace(&cap[0], &cred.token);
                }
                *value = new_val;
            }
        }
    }
    Ok(())
}

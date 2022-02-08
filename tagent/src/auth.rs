use std::error::Error;
use actix_web::{dev, HttpRequest};
use actix_web::error::ErrorBadRequest;
use actix_web::http::header;
use log::{debug, error, info};
use jwt_simple::algorithms::RS256PublicKey;
use jwt_simple::claims::NoCustomClaims;
use jwt_simple::prelude::*;
use serde::{Serialize, Deserialize};



// Tapis Tenants API structs ---

#[derive(Debug, Serialize, Deserialize)]
struct TapisTenantsResult {
    public_key: String,
    status: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct TenantsAPIResponse {
    result: TapisTenantsResult
}


// JWT claims ---
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    jti: String,
    iss: String,
    exp: usize,
}


// fetch the public key from a GET request to a uri.
// In pratcice, uri will be the Tapis tenants API endpoint; e.g., 
// uri = https://admin.tapis.io/v3/tenants/admin
async fn fetch_publickey(uri: &str) -> Result<String, String> {
    let res = reqwest::get(uri).await;
    let res = match res {
        Ok(response) => response,
        Err(error) => {
            let msg = format!("Got error from GET request to Tenants API to retrieve public key. error: {}", error);
            error!("{}", msg);
            return Err(msg);
        },
    };
    match res.status() {
        reqwest::StatusCode::OK => {
            info!("got 200 from request to fetch public key");
            match res.json::<TenantsAPIResponse>().await{
                Ok(response) => {
                    let public_key = response.result.public_key;
                    info!("Parsed JSON response from tenants API; public_key: {:?}", public_key);
                    return Ok(public_key);
                },
                Err(error) => {
                    let msg = format!("Could not parse the JSON response from the tenants API; err: {}", error);
                    error!("{}", msg);
                    return Err(msg);
                }
            }
        },
        other => {
            let msg = format!("did not get 200 from request to fetch public key; status: {}", res.status());
            error!("{}", msg);
            return Err(msg);
        }
    }
}



//get the value of a header_name from a request
// cf., https://stackoverflow.com/questions/52919494/is-there-simpler-method-to-get-the-string-value-of-an-actix-web-http-header
fn get_header_value<'a>(req: &'a dev::ServiceRequest, header_name: &str) -> Option<&'a str> {    
    req.headers().get(header_name)?.to_str().ok()
}


pub async fn get_subject_of_request(req: dev::ServiceRequest, 
                                    base_url: String, 
                                    tenant_id: String) -> Result<String, String>{

    let headers = req.headers();
    let token = get_header_value(&req, "x-tapis-token");
    let token = match token {
        Some(tok) => tok,
        None => return Err("no tapis token header found".to_string())
    };
    let url = format!("{}/v3/tenants/{}", base_url, tenant_id);
    let pub_key = fetch_publickey(&url).await;
    let pub_key = match pub_key {
        Ok(pk) => pk,
        Err(err) => return Err(err)
    };
    let rsa_pub_key = RS256PublicKey::from_pem(&pub_key);
    let rsa_pub_key = match rsa_pub_key {
        Ok(key) => key,
        Err(error) => {
            let msg = format!("Error generating RSAPublicKey from pub_key string; err: {}", error);
            error!("{}", msg);
            return Err(msg);
        }
    };
    // validate token using public key; get claims    
    let claims = rsa_pub_key.verify_token::<NoCustomClaims>(&token, None);
    let claims = match claims {
        Ok(claims) => claims,
        Err(error) => {
            let msg = format!("Error parsing token for claims; err: {}", error);
            error!("{}", msg);
            return Err(msg);
        }
    };
    match claims.subject {
        Some(sub) => return Ok(sub),
        None => {
            let msg = format!("token claims did not have a subject!");
            error!("{}", msg);
            return Err(msg);
        }
    };
}
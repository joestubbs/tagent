use actix_web::HttpRequest;
use jwt_simple::algorithms::RS256PublicKey;
use jwt_simple::claims::NoCustomClaims;
use jwt_simple::prelude::*;
use log::{debug, error};
use serde::{Deserialize, Serialize};

// JWT claims ---
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    jti: String,
    iss: String,
    exp: usize,
}

// get the value of a header_name from a request
// cf., https://stackoverflow.com/questions/52919494/is-there-simpler-method-to-get-the-string-value-of-an-actix-web-http-header
fn get_header_value<'a>(req: &'a HttpRequest, header_name: &str) -> Option<&'a str> {
    req.headers().get(header_name)?.to_str().ok()
}

pub async fn get_subject_of_request(
    req: HttpRequest,
    pub_key: &RS256PublicKey,
) -> Result<String, String> {
    debug!("top of get_subject_of_request");
    let token = get_header_value(&req, "x-tapis-token");
    debug!("returned from get_header_value..");
    let token = match token {
        Some(tok) => tok,
        None => return Err("no tapis token header found".to_string()),
    };
    debug!("got token from header, {}", token);
    // validate token using public key; get claims
    let claims = pub_key.verify_token::<NoCustomClaims>(token, None);
    let claims = match claims {
        Ok(claims) => claims,
        Err(error) => {
            let msg = format!("Error parsing token for claims; err: {}", error);
            error!("{}", msg);
            return Err(msg);
        }
    };
    match claims.subject {
        Some(sub) => Ok(sub),
        None => {
            let msg = "token claims did not have a subject!".to_string();
            error!("{}", msg);
            Err(msg)
        }
    }
}

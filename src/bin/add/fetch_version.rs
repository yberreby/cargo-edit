use rustc_serialize::json;
use rustc_serialize::json::{BuilderError, Json};
use curl::{ErrCode, http};
use curl::http::handle::{Method, Request};

const REGISTRY_HOST: &'static str = "https://crates.io";

pub fn get_latest_version(crate_name: &str) -> Result<String, FetchVersionError> {
    let crate_data = try!(fetch(&format!("/crates/{}", crate_name)));
    let crate_json = try!(Json::from_str(&crate_data));

    crate_json.as_object()
              .and_then(|c| c.get("crate"))
              .and_then(|c| c.as_object())
              .and_then(|c| c.get("max_version"))
              .and_then(|v| v.as_string())
              .map(|v| v.to_owned())
              .ok_or(FetchVersionError::GetVersion)
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchVersionError {
        CratesIo(err: CratesIoError) {
            from()
            description("crates.io Error")
            display("crates.io Error: {}", err)
            cause(err)
        }
        Json(err: BuilderError) {
            from()
            description("JSON Error")
            display("Error parsing JSON: {}", err)
            cause(err)
        }
        GetVersion { description("get version error") }
    }
}

// ---
// The following was mostly copied from [1] and is therefore
// (c) 2015 Alex Crichton <alex@alexcrichton.com>
//
// [1]: https://github.com/rust-lang/cargo/blob/bd690d8dff83c7b7714f236a08304ee20732382b/src/crates-io/lib.rs
// ---

fn fetch(path: &str) -> Result<String, FetchVersionError> {
    let mut http_handle = http::Handle::new();
    let req = Request::new(&mut http_handle, Method::Get)
                  .uri(format!("{}/api/v1{}", REGISTRY_HOST, path))
                  .header("Accept", "application/json")
                  .content_type("application/json");
    handle(req.exec()).map_err(From::from)
}

fn handle(response: Result<http::Response, ErrCode>) -> Result<String, CratesIoError> {
    let response = try!(response.map_err(CratesIoError::Curl));
    match response.get_code() {
        0 => {} // file upload url sometimes
        200 => {}
        403 => return Err(CratesIoError::Unauthorized),
        404 => return Err(CratesIoError::NotFound),
        _ => return Err(CratesIoError::NotOkResponse(response)),
    }

    let body = match String::from_utf8(response.move_body()) {
        Ok(body) => body,
        Err(..) => return Err(CratesIoError::NonUtf8Body),
    };
    match json::decode::<ApiErrorList>(&body) {
        Ok(errors) => {
            return Err(CratesIoError::Api(errors.errors.into_iter().map(|s| s.detail).collect()))
        }
        Err(..) => {}
    }
    Ok(body)
}

#[derive(RustcDecodable)]
struct ApiErrorList {
    errors: Vec<ApiError>,
}
#[derive(RustcDecodable)]
struct ApiError {
    detail: String,
}

quick_error! {
    #[derive(Debug)]
    pub enum CratesIoError {
        Curl(e: ErrCode) {}
        NotOkResponse(e: http::Response)  {}
        NonUtf8Body  {}
        Api(e: Vec<String>)  {}
        Unauthorized  {}
        NotFound {}
    }
}

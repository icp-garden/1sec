use pocket_ic::{
    common::rest::{
        CanisterHttpHeader, CanisterHttpMethod, CanisterHttpReply, CanisterHttpRequest,
        CanisterHttpResponse, MockCanisterHttpResponse, SubnetId,
    },
    nonblocking::PocketIc,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, path::PathBuf, sync::Weak};
use tokio::sync::Mutex;

use crate::WORKSPACE_ROOT;

/// This function run a loop that fetches pending HTTP outcalls from pocket-ic
/// and handles them either by forward them to anvil or by replaying responses
/// that were recorded by `record_http_responses` below.
pub async fn handle_http_outcalls(
    pocket_ic: Weak<Mutex<PocketIc>>,
    anvil: reqwest::Url,
    rpc_nodes: Vec<String>,
) {
    let recorded = RecordedHttp::load().unwrap_or_default();
    while let Some(pic) = pocket_ic.upgrade() {
        let requests = {
            let pic = pic.lock().await;
            pic.get_canister_http().await
        };
        for request in requests {
            let mut url = request.url.clone();
            if url.ends_with('/') {
                url.pop();
            }
            if rpc_nodes.contains(&url) {
                let response = forward_http(request, anvil.to_string()).await;
                let pic = pic.lock().await;
                pic.mock_canister_http_response(response).await;
            } else if let Some(reply) = recorded.lookup(url.clone()) {
                let response = forward_recorded(request, reply);
                let pic = pic.lock().await;
                pic.mock_canister_http_response(response).await;
            } else {
                let recorded_request: RecordedRequest = request.into();
                let json_str = serde_json::to_string(&recorded_request).unwrap();
                println!("MISSING {},", json_str);
            }
        }
    }
}

/// if you see "MISSING" output when running tests, then this means that there
/// are new unrecorded URLS. To record them:
/// - Copy the text after MISSING into `http_requests.json`.
/// - Run `cargo test record_http_responses -- --include-ignored`
#[ignore = "it is just a helper to record HTTP outcalls manually."]
#[tokio::test]
async fn record_http_responses() {
    let mut filename = RecordedHttp::filename();
    filename.pop();
    filename.push("http_requests.json");

    let mut recorded = RecordedHttp::load().unwrap_or_default();

    let mut file = std::fs::File::open(filename).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let requests: Vec<RecordedRequest> = serde_json::from_str(&data).unwrap();
    for request in requests {
        let url = request.url.clone();
        if recorded.lookup(url.clone()).is_none() {
            let response = forward_http(request.into(), url.clone()).await;
            recorded.record(url, response.response.clone().into());
        }
    }
    recorded.save();
}

/////////////////////////////////////////////////////
/// Everything that follows is private in the module.
/////////////////////////////////////////////////////

#[derive(minicbor::Encode, minicbor::Decode, Default)]
struct RecordedHttp {
    #[n(0)]
    reply_by_url: BTreeMap<String, RecordedReply>,
}

impl RecordedHttp {
    fn load() -> Option<Self> {
        let filename = Self::filename();
        let bytes = std::fs::read(filename.as_path()).ok()?;
        Some(minicbor::decode(&bytes).unwrap())
    }

    fn save(&self) {
        let filename = Self::filename();
        let mut bytes = vec![];
        minicbor::encode(self, &mut bytes).unwrap();
        std::fs::write(filename.as_path(), bytes).unwrap();
    }

    fn filename() -> PathBuf {
        let mut result = PathBuf::new();
        result.push(WORKSPACE_ROOT.clone());
        result.push("tests");
        result.push("src");
        result.push("helpers");
        result.push("http_responses.bin");
        result
    }

    fn lookup(&self, url: String) -> Option<RecordedReply> {
        let url = Self::sanitize(url);
        self.reply_by_url.get(&url).cloned()
    }

    fn record(&mut self, url: String, reply: RecordedReply) {
        let url = Self::sanitize(url);
        self.reply_by_url.insert(url, reply);
    }

    fn sanitize(url: String) -> String {
        url.chars().filter(|c| !c.is_ascii_digit()).collect()
    }
}

#[derive(minicbor::Encode, minicbor::Decode, Clone)]
struct RecordedReply {
    #[n(0)]
    status: u16,
    #[n(1)]
    headers: Vec<(String, String)>,
    #[n(2)]
    #[cbor(with = "minicbor::bytes")]
    body: Vec<u8>,
}

impl From<CanisterHttpReply> for RecordedReply {
    fn from(value: CanisterHttpReply) -> Self {
        Self {
            status: value.status,
            headers: headers_to_strings(value.headers),
            body: value.body,
        }
    }
}

impl From<CanisterHttpResponse> for RecordedReply {
    fn from(value: CanisterHttpResponse) -> Self {
        match value {
            CanisterHttpResponse::CanisterHttpReply(reply) => reply.into(),
            CanisterHttpResponse::CanisterHttpReject(_) => {
                panic!("Unexpected CanisterHttpReject, cannot convert it into RecordedReply");
            }
        }
    }
}

impl From<RecordedReply> for CanisterHttpReply {
    fn from(val: RecordedReply) -> Self {
        CanisterHttpReply {
            status: val.status,
            headers: strings_to_headers(val.headers),
            body: val.body,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RecordedRequest {
    url: String,
    headers: Vec<(String, String)>,
}

impl From<CanisterHttpRequest> for RecordedRequest {
    fn from(value: CanisterHttpRequest) -> Self {
        Self {
            url: value.url,
            headers: headers_to_strings(value.headers),
        }
    }
}

impl From<RecordedRequest> for CanisterHttpRequest {
    fn from(val: RecordedRequest) -> Self {
        CanisterHttpRequest {
            subnet_id: SubnetId::anonymous(),
            request_id: 0,
            http_method: CanisterHttpMethod::GET,
            url: val.url,
            headers: strings_to_headers(val.headers),
            body: vec![],
            max_response_bytes: None,
        }
    }
}

async fn forward_http(request: CanisterHttpRequest, url: String) -> MockCanisterHttpResponse {
    let client = reqwest::Client::new();

    let method = match request.http_method {
        CanisterHttpMethod::GET => reqwest::Method::GET,
        CanisterHttpMethod::POST => reqwest::Method::POST,
        CanisterHttpMethod::HEAD => reqwest::Method::HEAD,
    };

    let mut forward = client.request(method, url);
    for header in &request.headers {
        forward = forward.header(&header.name, &header.value);
    }
    forward = forward.body(request.body.clone());

    let outcome = forward.send().await;
    let Ok(response) = outcome else {
        return MockCanisterHttpResponse {
            subnet_id: request.subnet_id,
            request_id: request.request_id,
            response: CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply {
                status: 101,
                headers: vec![],
                body: vec![],
            }),
            additional_responses: vec![],
        };
    };

    let headers = strings_to_headers(
        response
            .headers()
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_str().unwrap().to_string()))
            .collect(),
    );

    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();

    MockCanisterHttpResponse {
        subnet_id: request.subnet_id,
        request_id: request.request_id,
        response: CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply {
            status,
            headers,
            body: bytes.to_vec(),
        }),
        additional_responses: vec![],
    }
}

fn forward_recorded(
    request: CanisterHttpRequest,
    reply: RecordedReply,
) -> MockCanisterHttpResponse {
    MockCanisterHttpResponse {
        subnet_id: request.subnet_id,
        request_id: request.request_id,
        response: CanisterHttpResponse::CanisterHttpReply(reply.into()),
        additional_responses: vec![],
    }
}

fn strings_to_headers(hs: Vec<(String, String)>) -> Vec<CanisterHttpHeader> {
    hs.into_iter()
        .map(|(name, value)| CanisterHttpHeader { name, value })
        .collect()
}

fn headers_to_strings(hs: Vec<CanisterHttpHeader>) -> Vec<(String, String)> {
    hs.into_iter().map(|h| (h.name, h.value)).collect()
}

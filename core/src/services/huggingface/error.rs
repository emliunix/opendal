// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::fmt::Debug;

use http::Response;
use http::StatusCode;
use serde::Deserialize;

use crate::raw::*;
use crate::*;

/// HuggingfaceError is the error returned by Huggingface File System.
#[derive(Default, Deserialize)]
struct HuggingfaceError {
    error: String,
}

impl Debug for HuggingfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut de = f.debug_struct("HuggingfaceError");
        de.field("message", &self.error.replace('\n', " "));

        de.finish()
    }
}

pub(super) fn parse_error(resp: Response<Buffer>) -> Error {
    let (parts, body) = resp.into_parts();
    let bs = body.to_bytes();

    let (kind, retryable) = match parts.status {
        StatusCode::NOT_FOUND => (ErrorKind::NotFound, false),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => (ErrorKind::PermissionDenied, false),
        StatusCode::PRECONDITION_FAILED => (ErrorKind::ConditionNotMatch, false),
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => (ErrorKind::Unexpected, true),
        _ => (ErrorKind::Unexpected, false),
    };

    let message = match serde_json::from_slice::<HuggingfaceError>(&bs) {
        Ok(hf_error) => format!("{:?}", hf_error.error),
        Err(_) => String::from_utf8_lossy(&bs).into_owned(),
    };

    let mut err = Error::new(kind, message);

    err = with_error_response_context(err, parts);

    if retryable {
        err = err.set_temporary();
    }

    err
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::raw::new_json_deserialize_error;
    use crate::types::Result;

    #[test]
    fn test_parse_error() -> Result<()> {
        let resp = r#"
            {
                "error": "Invalid username or password."
            }
            "#;
        let decoded_response = serde_json::from_slice::<HuggingfaceError>(resp.as_bytes())
            .map_err(new_json_deserialize_error)?;

        assert_eq!(decoded_response.error, "Invalid username or password.");

        Ok(())
    }
}

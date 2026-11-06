//! Bound response allocation independently of Content-Length (which may be absent or wrong).
pub async fn read_body(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(format!(
            "Response exceeds the {} MiB download limit",
            limit / 1024 / 1024
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read the response body".to_string())?
    {
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err(format!(
                "Response exceeds the {} MiB download limit",
                limit / 1024 / 1024
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
